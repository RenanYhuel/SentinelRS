use std::fs;
use std::io;
use std::path::Path;

pub struct WalStats {
    pub total_size_bytes: u64,
    pub segment_count: u64,
    pub unacked_count: u64,
}

pub fn compute_stats(dir: &Path, unacked_count: u64) -> io::Result<WalStats> {
    let mut total_size_bytes = 0u64;
    let mut segment_count = 0u64;

    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "log").unwrap_or(false) {
                total_size_bytes += fs::metadata(&path)?.len();
                segment_count += 1;
            }
        }
    }

    Ok(WalStats {
        total_size_bytes,
        segment_count,
        unacked_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dir_returns_zero_stats() {
        let dir = tempfile::tempdir().unwrap();
        let stats = compute_stats(dir.path(), 0).unwrap();
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.segment_count, 0);
        assert_eq!(stats.unacked_count, 0);
    }

    #[test]
    fn counts_only_log_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("wal-0000000.log"), b"data").unwrap();
        fs::write(dir.path().join("wal.meta.json"), b"{}").unwrap();
        let stats = compute_stats(dir.path(), 5).unwrap();
        assert_eq!(stats.segment_count, 1);
        assert_eq!(stats.total_size_bytes, 4);
        assert_eq!(stats.unacked_count, 5);
    }
}
