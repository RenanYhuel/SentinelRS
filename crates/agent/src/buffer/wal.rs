use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::meta::WalMeta;
use super::record::Record;
use super::segment::Segment;

pub struct Wal {
    dir: PathBuf,
    current: Segment,
    segment_index: u64,
    next_id: u64,
    acked: HashSet<u64>,
    fsync: bool,
    max_segment_bytes: u64,
}

impl Wal {
    pub fn open(dir: &Path, fsync: bool, max_segment_bytes: u64) -> io::Result<Self> {
        fs::create_dir_all(dir)?;

        let meta = WalMeta::load(dir)?;
        let mut segment_index = meta.last_segment;
        let mut next_id = meta.tail_seq;
        let acked: HashSet<u64> = meta.acked_set();

        let mut entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort_by_key(|e| e.path());

        for entry in &entries {
            if let Ok(records) = Segment::read_all(&entry.path()) {
                for r in &records {
                    if r.id >= next_id {
                        next_id = r.id + 1;
                    }
                }
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(idx_str) = name.strip_prefix("wal-").and_then(|s| s.strip_suffix(".log")) {
                if let Ok(idx) = idx_str.parse::<u64>() {
                    if idx >= segment_index {
                        segment_index = idx + 1;
                    }
                }
            }
        }

        let current = Segment::create(dir, segment_index)?;

        Ok(Self {
            dir: dir.to_path_buf(),
            current,
            segment_index,
            next_id,
            acked,
            fsync,
            max_segment_bytes,
        })
    }

    pub fn append(&mut self, data: Vec<u8>) -> io::Result<u64> {
        let id = self.next_id;
        self.next_id += 1;

        let record = Record { id, data };

        if let Ok(size) = self.current.size_bytes() {
            if size >= self.max_segment_bytes {
                self.rotate()?;
            }
        }

        self.current.append(&record, self.fsync)?;
        Ok(id)
    }

    pub fn ack(&mut self, record_id: u64) {
        self.acked.insert(record_id);
    }

    pub fn iter_unacked(&self) -> io::Result<Vec<(u64, Vec<u8>)>> {
        let mut results = Vec::new();
        let mut entries: Vec<_> = fs::read_dir(&self.dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            if let Ok(records) = Segment::read_all(&entry.path()) {
                for r in records {
                    if !self.acked.contains(&r.id) {
                        results.push((r.id, r.data));
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn unacked_count(&self) -> io::Result<usize> {
        Ok(self.iter_unacked()?.len())
    }

    pub fn save_meta(&self) -> io::Result<WalMeta> {
        let head = self.head_seq();
        let meta = WalMeta {
            head_seq: head,
            tail_seq: self.next_id,
            last_segment: self.segment_index,
            acked_ids: self.acked.iter().copied().collect(),
        };
        meta.save(&self.dir)?;
        Ok(meta)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn head_seq(&self) -> u64 {
        if self.acked.is_empty() {
            return 0;
        }
        let mut min_unacked = self.next_id;
        for id in 0..self.next_id {
            if !self.acked.contains(&id) {
                min_unacked = id;
                break;
            }
        }
        min_unacked
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.segment_index += 1;
        self.current = Segment::create(&self.dir, self.segment_index)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), true, 1024 * 1024).unwrap();

        let id0 = wal.append(b"record-0".to_vec()).unwrap();
        let id1 = wal.append(b"record-1".to_vec()).unwrap();

        let unacked = wal.iter_unacked().unwrap();
        assert_eq!(unacked.len(), 2);
        assert_eq!(unacked[0].0, id0);
        assert_eq!(unacked[1].0, id1);
    }

    #[test]
    fn ack_removes_from_unacked() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), true, 1024 * 1024).unwrap();

        let id0 = wal.append(b"a".to_vec()).unwrap();
        let _id1 = wal.append(b"b".to_vec()).unwrap();
        wal.ack(id0);

        let unacked = wal.iter_unacked().unwrap();
        assert_eq!(unacked.len(), 1);
        assert_eq!(unacked[0].1, b"b");
    }

    #[test]
    fn recovery_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut wal = Wal::open(dir.path(), true, 1024 * 1024).unwrap();
            wal.append(b"persistent".to_vec()).unwrap();
            wal.ack(0);
            wal.save_meta().unwrap();
        }
        {
            let wal = Wal::open(dir.path(), true, 1024 * 1024).unwrap();
            let unacked = wal.iter_unacked().unwrap();
            assert_eq!(unacked.len(), 0);
        }
    }

    #[test]
    fn segment_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), true, 50).unwrap();

        for i in 0..10 {
            wal.append(format!("record-{}", i).into_bytes()).unwrap();
        }

        let logs: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .collect();
        assert!(logs.len() > 1, "should have rotated into multiple segments");
    }
}
