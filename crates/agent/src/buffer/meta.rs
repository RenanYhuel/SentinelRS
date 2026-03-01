use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const META_FILE: &str = "wal.meta.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalMeta {
    pub head_seq: u64,
    pub tail_seq: u64,
    pub last_segment: u64,
    pub acked_ids: Vec<u64>,
}

impl WalMeta {
    pub fn empty() -> Self {
        Self {
            head_seq: 0,
            tail_seq: 0,
            last_segment: 0,
            acked_ids: Vec::new(),
        }
    }

    pub fn unacked_count(&self) -> u64 {
        let total_records = self.tail_seq - self.head_seq;
        total_records.saturating_sub(self.acked_ids.len() as u64)
    }

    pub fn acked_set(&self) -> HashSet<u64> {
        self.acked_ids.iter().copied().collect()
    }

    pub fn load(dir: &Path) -> io::Result<Self> {
        let path = dir.join(META_FILE);
        if !path.exists() {
            return Ok(Self::empty());
        }
        let data = fs::read_to_string(&path)?;
        serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        let path = dir.join(META_FILE);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let mut f = fs::File::create(&path)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_meta_defaults() {
        let m = WalMeta::empty();
        assert_eq!(m.head_seq, 0);
        assert_eq!(m.tail_seq, 0);
        assert_eq!(m.unacked_count(), 0);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = WalMeta::empty();
        m.head_seq = 0;
        m.tail_seq = 10;
        m.last_segment = 3;
        m.acked_ids = vec![0, 1, 2];
        m.save(dir.path()).unwrap();

        let loaded = WalMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.head_seq, 0);
        assert_eq!(loaded.tail_seq, 10);
        assert_eq!(loaded.last_segment, 3);
        assert_eq!(loaded.acked_ids.len(), 3);
    }

    #[test]
    fn unacked_count_calculation() {
        let mut m = WalMeta::empty();
        m.head_seq = 5;
        m.tail_seq = 15;
        m.acked_ids = vec![5, 7, 9];
        assert_eq!(m.unacked_count(), 7);
    }

    #[test]
    fn load_missing_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let m = WalMeta::load(dir.path()).unwrap();
        assert_eq!(m.tail_seq, 0);
    }

    #[test]
    fn save_produces_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let m = WalMeta {
            head_seq: 0,
            tail_seq: 5,
            last_segment: 1,
            acked_ids: vec![0, 2],
        };
        m.save(dir.path()).unwrap();
        assert!(dir.path().join("wal.meta.json").exists());
        let loaded = WalMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.tail_seq, 5);
    }
}
