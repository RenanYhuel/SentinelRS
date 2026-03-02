use std::io;
use std::path::Path;

use crate::buffer::Wal;

use super::state::AgentPersistedState;
use super::volume::{self, VolumeCheck, VolumeLayout};

#[derive(Debug)]
pub struct RecoveryResult {
    pub state: AgentPersistedState,
    pub resume_seq: u64,
    pub pending_batches: usize,
    pub was_clean_shutdown: bool,
    pub first_boot: bool,
}

pub fn check_volume(layout: &VolumeLayout, wal_dir: Option<&str>) -> io::Result<VolumeCheck> {
    let check = volume::verify(layout, wal_dir)?;

    if !check.root_exists || !check.root_writable {
        tracing::warn!(
            target: "boot",
            root = %layout.root().display(),
            exists = check.root_exists,
            writable = check.root_writable,
            "Volume not ready, initializing"
        );
        volume::initialize(layout, wal_dir)?;
        return volume::verify(layout, wal_dir);
    }

    Ok(check)
}

pub fn recover_state(
    state_dir: &Path,
    agent_id: &str,
    server_url: &str,
) -> io::Result<(AgentPersistedState, bool)> {
    match AgentPersistedState::load(state_dir)? {
        Some(mut state) => {
            let clean = state.clean_shutdown;
            state.record_boot();
            state.save(state_dir)?;

            tracing::info!(
                target: "boot",
                agent_id = %state.agent_id,
                boot_count = state.boot_count,
                prev_seq = state.seq_counter,
                clean_shutdown = clean,
                "Restored persisted state"
            );

            Ok((state, false))
        }
        None => {
            let mut state = AgentPersistedState::new(agent_id.into(), server_url.into());
            state.record_boot();
            state.save(state_dir)?;

            tracing::info!(target: "boot", agent_id, "First boot, created agent state");

            Ok((state, true))
        }
    }
}

pub fn recover_wal(wal: &Wal) -> io::Result<(u64, usize)> {
    let unacked = wal.iter_unacked()?;
    let pending = unacked.len();
    let resume_seq = wal.next_id();

    if pending > 0 {
        tracing::info!(
            target: "boot",
            pending,
            resume_seq,
            "Resuming from previous state. {} pending batches in WAL",
            pending
        );
    } else {
        tracing::info!(target: "boot", resume_seq, "WAL clean, no pending batches");
    }

    Ok((resume_seq, pending))
}

pub fn full_recovery(
    layout: &VolumeLayout,
    wal_dir_override: Option<&str>,
    agent_id: &str,
    server_url: &str,
    wal: &Wal,
) -> io::Result<RecoveryResult> {
    let _volume_check = check_volume(layout, wal_dir_override)?;
    let (mut state, first_boot) = recover_state(&layout.state_dir(), agent_id, server_url)?;
    let (resume_seq, pending_batches) = recover_wal(wal)?;

    let was_clean = state.clean_shutdown || first_boot;

    state.update_seq(resume_seq);
    state.save(&layout.state_dir())?;

    Ok(RecoveryResult {
        state,
        resume_seq,
        pending_batches,
        was_clean_shutdown: was_clean,
        first_boot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_layout(root: &Path) -> VolumeLayout {
        VolumeLayout::new(root)
    }

    #[test]
    fn check_volume_initializes_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sentinel");
        let layout = test_layout(&sub);
        let check = check_volume(&layout, None).unwrap();
        assert!(check.root_exists);
        assert!(check.root_writable);
    }

    #[test]
    fn recover_state_first_boot() {
        let tmp = tempfile::tempdir().unwrap();
        let (state, first) = recover_state(tmp.path(), "agent-1", "https://s").unwrap();
        assert!(first);
        assert_eq!(state.agent_id, "agent-1");
        assert_eq!(state.boot_count, 1);
        assert!(!state.clean_shutdown);
    }

    #[test]
    fn recover_state_subsequent_boot() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let mut s = AgentPersistedState::new("agent-1".into(), "https://s".into());
            s.seq_counter = 100;
            s.record_boot();
            s.record_shutdown();
            s.save(tmp.path()).unwrap();
        }
        let (state, first) = recover_state(tmp.path(), "agent-1", "https://s").unwrap();
        assert!(!first);
        assert_eq!(state.boot_count, 2);
        assert_eq!(state.seq_counter, 100);
    }

    #[test]
    fn recover_wal_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = Wal::open(tmp.path(), false, 1024 * 1024).unwrap();
        let (seq, pending) = recover_wal(&wal).unwrap();
        assert_eq!(seq, 0);
        assert_eq!(pending, 0);
    }

    #[test]
    fn recover_wal_with_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(tmp.path(), false, 1024 * 1024).unwrap();
        wal.append(b"batch1".to_vec()).unwrap();
        wal.append(b"batch2".to_vec()).unwrap();
        wal.ack(0);

        let (seq, pending) = recover_wal(&wal).unwrap();
        assert_eq!(seq, 2);
        assert_eq!(pending, 1);
    }

    #[test]
    fn full_recovery_first_boot() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = test_layout(tmp.path());
        volume::initialize(&layout, None).unwrap();
        let wal_path = layout.wal_dir(None);
        let wal = Wal::open(&wal_path, false, 1024 * 1024).unwrap();

        let result = full_recovery(&layout, None, "a1", "https://s", &wal).unwrap();
        assert!(result.first_boot);
        assert_eq!(result.pending_batches, 0);
        assert_eq!(result.resume_seq, 0);
    }

    #[test]
    fn full_recovery_after_crash() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = test_layout(tmp.path());
        volume::initialize(&layout, None).unwrap();

        {
            let mut s = AgentPersistedState::new("a1".into(), "https://s".into());
            s.seq_counter = 50;
            s.record_boot();
            s.save(&layout.state_dir()).unwrap();
        }

        let wal_path = layout.wal_dir(None);
        let mut wal = Wal::open(&wal_path, false, 1024 * 1024).unwrap();
        wal.append(b"pending1".to_vec()).unwrap();
        wal.append(b"pending2".to_vec()).unwrap();
        wal.save_meta().unwrap();

        drop(wal);
        let wal = Wal::open(&wal_path, false, 1024 * 1024).unwrap();

        let result = full_recovery(&layout, None, "a1", "https://s", &wal).unwrap();
        assert!(!result.first_boot);
        assert!(!result.was_clean_shutdown);
        assert_eq!(result.pending_batches, 2);
        assert_eq!(result.resume_seq, 2);
    }
}
