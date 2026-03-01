#[cfg(test)]
mod tests {
    use sentinel_agent::buffer::Wal;

    #[test]
    fn wal_inspect_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let wal = Wal::open(dir.path(), false, 16 * 1024 * 1024).unwrap();
        let entries = wal.iter_unacked().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn wal_stats_empty() {
        let dir = tempfile::tempdir().unwrap();
        let wal = Wal::open(dir.path(), false, 16 * 1024 * 1024).unwrap();
        let unacked = wal.unacked_count().unwrap() as u64;
        let stats = sentinel_agent::buffer::compute_stats(dir.path(), unacked).unwrap();
        assert_eq!(stats.unacked_count, 0);
    }

    #[test]
    fn wal_meta_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let meta = sentinel_agent::buffer::WalMeta::load(dir.path()).unwrap();
        assert_eq!(meta.head_seq, 0);
        assert_eq!(meta.tail_seq, 0);
    }

    #[test]
    fn wal_after_append() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), false, 16 * 1024 * 1024).unwrap();
        let id = wal.append(b"test-data".to_vec()).unwrap();
        assert!(id >= 0);
        let entries = wal.iter_unacked().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn wal_ack_removes_entry() {
        let dir = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(dir.path(), false, 16 * 1024 * 1024).unwrap();
        let id = wal.append(b"test-data".to_vec()).unwrap();
        wal.ack(id);
        let entries = wal.iter_unacked().unwrap();
        assert!(entries.is_empty());
    }
}
