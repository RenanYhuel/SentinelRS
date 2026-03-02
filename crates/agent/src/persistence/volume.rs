use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const STATE_DIR: &str = "state";
const WAL_SUBDIR: &str = "wal";

pub struct VolumeLayout {
    root: PathBuf,
}

impl VolumeLayout {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.yml")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.root.join(STATE_DIR)
    }

    pub fn state_file(&self) -> PathBuf {
        self.state_dir().join("agent.state.json")
    }

    pub fn wal_dir(&self, custom: Option<&str>) -> PathBuf {
        match custom {
            Some(d) => PathBuf::from(d),
            None => self.root.join(WAL_SUBDIR),
        }
    }
}

#[derive(Debug)]
pub struct VolumeCheck {
    pub root_exists: bool,
    pub root_writable: bool,
    pub config_exists: bool,
    pub state_exists: bool,
    pub wal_exists: bool,
}

pub fn verify(layout: &VolumeLayout, wal_dir: Option<&str>) -> io::Result<VolumeCheck> {
    let root_exists = layout.root().exists();
    let config_exists = layout.config_file().exists();
    let state_exists = layout.state_file().exists();
    let wal_path = layout.wal_dir(wal_dir);
    let wal_exists = wal_path.exists();
    let root_writable = if root_exists {
        is_writable(layout.root())
    } else {
        false
    };

    Ok(VolumeCheck {
        root_exists,
        root_writable,
        config_exists,
        state_exists,
        wal_exists,
    })
}

pub fn initialize(layout: &VolumeLayout, wal_dir: Option<&str>) -> io::Result<()> {
    fs::create_dir_all(layout.root())?;
    fs::create_dir_all(layout.state_dir())?;
    fs::create_dir_all(layout.wal_dir(wal_dir))?;

    #[cfg(unix)]
    set_unix_permissions(layout.root())?;

    if !is_writable(layout.root()) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Volume root is not writable: {}", layout.root().display()),
        ));
    }

    Ok(())
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".sentinel_probe");
    match fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(unix)]
fn set_unix_permissions(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o750);
    fs::set_permissions(dir, perms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_paths() {
        let layout = VolumeLayout::new(Path::new("/etc/sentinel"));
        assert_eq!(layout.config_file(), PathBuf::from("/etc/sentinel/config.yml"));
        assert_eq!(layout.state_file(), PathBuf::from("/etc/sentinel/state/agent.state.json"));
        assert_eq!(layout.wal_dir(None), PathBuf::from("/etc/sentinel/wal"));
        assert_eq!(
            layout.wal_dir(Some("/custom/wal")),
            PathBuf::from("/custom/wal")
        );
    }

    #[test]
    fn initialize_creates_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = VolumeLayout::new(tmp.path());
        initialize(&layout, None).unwrap();
        assert!(layout.root().exists());
        assert!(layout.state_dir().exists());
        assert!(layout.wal_dir(None).exists());
    }

    #[test]
    fn verify_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = VolumeLayout::new(tmp.path());
        let check = verify(&layout, None).unwrap();
        assert!(check.root_exists);
        assert!(check.root_writable);
        assert!(!check.config_exists);
        assert!(!check.state_exists);
        assert!(!check.wal_exists);
    }

    #[test]
    fn verify_after_init() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = VolumeLayout::new(tmp.path());
        initialize(&layout, None).unwrap();
        let check = verify(&layout, None).unwrap();
        assert!(check.root_exists);
        assert!(check.root_writable);
        assert!(!check.config_exists);
        assert!(!check.state_exists);
        assert!(check.wal_exists);
    }
}
