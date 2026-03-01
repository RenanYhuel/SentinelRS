use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use super::meta::WalMeta;
use super::record::Record;
use super::segment::Segment;

pub fn compact(dir: &Path, meta: &WalMeta) -> io::Result<WalMeta> {
    let acked = meta.acked_set();
    let unacked_records = collect_unacked(dir, &acked)?;

    let tmp_dir = dir.join(".compact_tmp");
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir)?;
    }
    fs::create_dir_all(&tmp_dir)?;

    let new_segment_index = 0u64;
    let mut segment = Segment::create(&tmp_dir, new_segment_index)?;
    for record in &unacked_records {
        segment.append(record, true)?;
    }

    remove_log_files(dir)?;

    for entry in fs::read_dir(&tmp_dir)? {
        let entry = entry?;
        let dest = dir.join(entry.file_name());
        fs::rename(entry.path(), &dest)?;
    }
    fs::remove_dir_all(&tmp_dir)?;

    let head_seq = unacked_records.first().map(|r| r.id).unwrap_or(meta.tail_seq);
    let new_meta = WalMeta {
        head_seq,
        tail_seq: meta.tail_seq,
        last_segment: new_segment_index,
        acked_ids: Vec::new(),
    };
    new_meta.save(dir)?;
    Ok(new_meta)
}

pub fn needs_compaction(dir: &Path, threshold_bytes: u64) -> io::Result<bool> {
    let total = total_log_size(dir)?;
    Ok(total >= threshold_bytes)
}

fn collect_unacked(dir: &Path, acked: &HashSet<u64>) -> io::Result<Vec<Record>> {
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

    let mut result = Vec::new();
    for entry in entries {
        if let Ok(records) = Segment::read_all(&entry.path()) {
            for r in records {
                if !acked.contains(&r.id) {
                    result.push(r);
                }
            }
        }
    }
    Ok(result)
}

fn remove_log_files(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "log").unwrap_or(false) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn total_log_size(dir: &Path) -> io::Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "log").unwrap_or(false) {
            total += fs::metadata(&path)?.len();
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::wal::Wal;

    #[test]
    fn compact_removes_acked_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), true, 1024 * 1024).unwrap();

        let _id0 = wal.append(b"keep-me".to_vec()).unwrap();
        let id1 = wal.append(b"ack-me".to_vec()).unwrap();
        let _id2 = wal.append(b"also-keep".to_vec()).unwrap();

        wal.ack(id1);
        let meta = wal.save_meta().unwrap();

        let new_meta = compact(dir.path(), &meta).unwrap();
        assert!(new_meta.acked_ids.is_empty());

        let wal2 = Wal::open(dir.path(), true, 1024 * 1024).unwrap();
        let unacked = wal2.iter_unacked().unwrap();
        assert_eq!(unacked.len(), 2);
        assert_eq!(unacked[0].1, b"keep-me");
        assert_eq!(unacked[1].1, b"also-keep");
    }

    #[test]
    fn compact_all_acked_produces_empty_wal() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), true, 1024 * 1024).unwrap();

        let id0 = wal.append(b"a".to_vec()).unwrap();
        let id1 = wal.append(b"b".to_vec()).unwrap();
        wal.ack(id0);
        wal.ack(id1);
        let meta = wal.save_meta().unwrap();

        let new_meta = compact(dir.path(), &meta).unwrap();
        assert_eq!(new_meta.acked_ids.len(), 0);

        let wal2 = Wal::open(dir.path(), true, 1024 * 1024).unwrap();
        assert_eq!(wal2.iter_unacked().unwrap().len(), 0);
    }

    #[test]
    fn needs_compaction_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), true, 1024 * 1024).unwrap();
        for i in 0..50 {
            wal.append(format!("record-{i}").into_bytes()).unwrap();
        }
        assert!(needs_compaction(dir.path(), 100).unwrap());
        assert!(!needs_compaction(dir.path(), 1024 * 1024).unwrap());
    }

    #[test]
    fn compact_preserves_record_ordering() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), true, 50).unwrap();

        let mut ids = Vec::new();
        for i in 0..10 {
            ids.push(wal.append(format!("rec-{i}").into_bytes()).unwrap());
        }
        wal.ack(ids[1]);
        wal.ack(ids[3]);
        wal.ack(ids[5]);
        let meta = wal.save_meta().unwrap();

        compact(dir.path(), &meta).unwrap();

        let wal2 = Wal::open(dir.path(), true, 1024 * 1024).unwrap();
        let unacked = wal2.iter_unacked().unwrap();
        assert_eq!(unacked.len(), 7);
        assert_eq!(unacked[0].1, b"rec-0");
        assert_eq!(unacked[1].1, b"rec-2");
        assert_eq!(unacked[2].1, b"rec-4");
    }
}
