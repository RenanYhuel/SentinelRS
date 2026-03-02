use std::path::Path;

pub fn write_config(config_dir: &Path, config_yaml: &[u8]) -> Result<(), WriteError> {
    std::fs::create_dir_all(config_dir).map_err(|e| WriteError::Io(e.to_string()))?;

    let config_path = config_dir.join("config.yml");

    if config_path.exists() {
        return Err(WriteError::AlreadyExists);
    }

    std::fs::write(&config_path, config_yaml).map_err(|e| WriteError::Io(e.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(&config_path, perms);
    }

    tracing::info!(path = %config_path.display(), "bootstrap config written");
    Ok(())
}

#[derive(Debug)]
pub enum WriteError {
    Io(String),
    AlreadyExists,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::AlreadyExists => write!(f, "config file already exists"),
        }
    }
}

impl std::error::Error for WriteError {}
