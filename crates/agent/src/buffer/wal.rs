use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

        let mut segment_index = 0u64;
        let mut next_id = 0u64;
        let mut acked = HashSet::new();

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

        if let Ok(meta) = fs::read_to_string(dir.join("wal.acked.json")) {
            if let Ok(ids) = serde_json::from_str::<Vec<u64>>(&meta) {
                acked.extend(ids);
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

    pub fn persist_acked(&self) -> io::Result<()> {
        let ids: Vec<u64> = self.acked.iter().copied().collect();
        let json = serde_json::to_string(&ids)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(self.dir.join("wal.acked.json"), json)
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
            wal.persist_acked().unwrap();
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
