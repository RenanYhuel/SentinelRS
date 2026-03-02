use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const STATE_FILE: &str = "agent.state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersistedState {
    pub agent_id: String,
    pub seq_counter: u64,
    pub boot_count: u64,
    pub last_connection_ms: i64,
    pub last_shutdown_ms: i64,
    pub key_id: String,
    pub server_url: String,
    #[serde(default)]
    pub clean_shutdown: bool,
}

impl AgentPersistedState {
    pub fn new(agent_id: String, server_url: String) -> Self {
        Self {
            agent_id,
            seq_counter: 0,
            boot_count: 0,
            last_connection_ms: 0,
            last_shutdown_ms: 0,
            key_id: "default".into(),
            server_url,
            clean_shutdown: true,
        }
    }

    pub fn load(dir: &Path) -> io::Result<Option<Self>> {
        let path = dir.join(STATE_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&path)?;
        let state: Self = serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(state))
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let path = dir.join(STATE_FILE);
        let tmp = dir.join(".agent.state.json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn record_boot(&mut self) {
        self.boot_count += 1;
        self.clean_shutdown = false;
    }

    pub fn record_shutdown(&mut self) {
        self.last_shutdown_ms = now_ms();
        self.clean_shutdown = true;
    }

    pub fn record_connection(&mut self) {
        self.last_connection_ms = now_ms();
    }

    pub fn update_seq(&mut self, seq: u64) {
        self.seq_counter = seq;
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_defaults() {
        let s = AgentPersistedState::new("test-agent".into(), "https://server".into());
        assert_eq!(s.agent_id, "test-agent");
        assert_eq!(s.seq_counter, 0);
        assert_eq!(s.boot_count, 0);
        assert!(s.clean_shutdown);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = AgentPersistedState::new("a1".into(), "https://s".into());
        s.seq_counter = 42;
        s.boot_count = 3;
        s.save(tmp.path()).unwrap();

        let loaded = AgentPersistedState::load(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.agent_id, "a1");
        assert_eq!(loaded.seq_counter, 42);
        assert_eq!(loaded.boot_count, 3);
    }

    #[test]
    fn load_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = AgentPersistedState::load(tmp.path()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn record_boot_increments() {
        let mut s = AgentPersistedState::new("a".into(), "s".into());
        s.record_boot();
        assert_eq!(s.boot_count, 1);
        assert!(!s.clean_shutdown);
        s.record_boot();
        assert_eq!(s.boot_count, 2);
    }

    #[test]
    fn record_shutdown_sets_clean() {
        let mut s = AgentPersistedState::new("a".into(), "s".into());
        s.record_boot();
        assert!(!s.clean_shutdown);
        s.record_shutdown();
        assert!(s.clean_shutdown);
        assert!(s.last_shutdown_ms > 0);
    }

    #[test]
    fn atomic_write() {
        let tmp = tempfile::tempdir().unwrap();
        let s = AgentPersistedState::new("a".into(), "s".into());
        s.save(tmp.path()).unwrap();
        assert!(!tmp.path().join(".agent.state.json.tmp").exists());
        assert!(tmp.path().join("agent.state.json").exists());
    }
}
