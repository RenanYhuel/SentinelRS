use std::path::{Path, PathBuf};

use super::error::PluginError;
use super::installer::{load_blob, verify_blob};
use super::manifest::PluginManifest;

pub struct DiscoveredPlugin {
    pub name: String,
    pub manifest: PluginManifest,
    pub wasm_bytes: Vec<u8>,
    pub path: PathBuf,
}

pub fn scan_plugins_dir(dir: &Path, signing_key: Option<&[u8]>) -> Vec<DiscoveredPlugin> {
    let mut plugins = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(target: "plugin", dir = %dir.display(), error = %e, "Cannot read plugins directory");
            return plugins;
        }
    };

    let mut manifest_names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = extract_manifest_name(&path) {
            manifest_names.push(name);
        }
    }

    manifest_names.sort();
    manifest_names.dedup();

    for name in manifest_names {
        match load_single_plugin(dir, &name, signing_key) {
            Ok(p) => {
                tracing::info!(target: "plugin", name = %p.name, version = %p.manifest.version, "Discovered plugin");
                plugins.push(p);
            }
            Err(e) => {
                tracing::warn!(target: "plugin", name = %name, error = %e, "Skipping plugin");
            }
        }
    }

    plugins
}

fn load_single_plugin(
    dir: &Path,
    name: &str,
    signing_key: Option<&[u8]>,
) -> Result<DiscoveredPlugin, PluginError> {
    let manifest_path = dir.join(format!("{name}.manifest.yml"));
    let manifest_yaml = std::fs::read_to_string(&manifest_path)?;
    let manifest = PluginManifest::from_yaml(&manifest_yaml)
        .map_err(|e| PluginError::InvalidOutput(format!("manifest parse: {e}")))?;

    let wasm_bytes = load_blob(dir, name)?;

    if let Some(key) = signing_key {
        let sig_path = dir.join(format!("{name}.sig"));
        let sig = std::fs::read(&sig_path).map_err(|e| {
            PluginError::InvalidOutput(format!(
                "missing signature file {}: {e}",
                sig_path.display()
            ))
        })?;
        if !verify_blob(&wasm_bytes, &sig, key) {
            return Err(PluginError::InvalidOutput(format!(
                "signature verification failed for {name}"
            )));
        }
    }

    Ok(DiscoveredPlugin {
        name: name.to_string(),
        manifest,
        wasm_bytes,
        path: dir.join(format!("{name}.wasm")),
    })
}

fn extract_manifest_name(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    file_name.strip_suffix(".manifest.yml").map(String::from)
}

pub fn list_installed(dir: &Path) -> Vec<(String, PluginManifest)> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = extract_manifest_name(&path) {
            let manifest_path = dir.join(format!("{name}.manifest.yml"));
            if let Ok(yaml) = std::fs::read_to_string(manifest_path) {
                if let Ok(m) = PluginManifest::from_yaml(&yaml) {
                    results.push((name, m));
                }
            }
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

pub fn remove_plugin(dir: &Path, name: &str) -> Result<(), PluginError> {
    let wasm_path = dir.join(format!("{name}.wasm"));
    let manifest_path = dir.join(format!("{name}.manifest.yml"));
    let sig_path = dir.join(format!("{name}.sig"));

    if !wasm_path.exists() && !manifest_path.exists() {
        return Err(PluginError::InvalidOutput(format!(
            "plugin '{name}' not found"
        )));
    }

    let _ = std::fs::remove_file(wasm_path);
    let _ = std::fs::remove_file(manifest_path);
    let _ = std::fs::remove_file(sig_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::installer::{sign_blob, store_blob, store_manifest};
    use crate::plugin::manifest::ResourceLimits;
    use std::collections::HashMap;

    fn test_wasm_wat() -> &'static str {
        r#"
            (module
                (memory (export "memory") 1)
                (func (export "collect") (result i32) (i32.const 0))
            )
        "#
    }

    fn test_manifest() -> PluginManifest {
        PluginManifest {
            name: "test".into(),
            version: "1.0.0".into(),
            entry_fn: "collect".into(),
            capabilities: vec![],
            resource_limits: ResourceLimits::default(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn scan_discovers_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = test_manifest();

        store_blob(dir.path(), "test", test_wasm_wat().as_bytes()).unwrap();
        store_manifest(dir.path(), "test", &manifest.to_yaml().unwrap()).unwrap();

        let plugins = scan_plugins_dir(dir.path(), None);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "test");
        assert_eq!(plugins[0].manifest.version, "1.0.0");
    }

    #[test]
    fn scan_with_signing_verification() {
        let dir = tempfile::tempdir().unwrap();
        let wasm = test_wasm_wat().as_bytes();
        let manifest = test_manifest();
        let key = b"signing-key";

        store_blob(dir.path(), "signed", wasm).unwrap();
        store_manifest(dir.path(), "signed", &manifest.to_yaml().unwrap()).unwrap();

        let sig = sign_blob(wasm, key);
        std::fs::write(dir.path().join("signed.sig"), &sig).unwrap();

        let plugins = scan_plugins_dir(dir.path(), Some(key));
        assert_eq!(plugins.len(), 1);
    }

    #[test]
    fn scan_rejects_bad_signature() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = test_manifest();

        store_blob(dir.path(), "bad", test_wasm_wat().as_bytes()).unwrap();
        store_manifest(dir.path(), "bad", &manifest.to_yaml().unwrap()).unwrap();
        std::fs::write(dir.path().join("bad.sig"), b"invalid-sig").unwrap();

        let plugins = scan_plugins_dir(dir.path(), Some(b"key"));
        assert!(plugins.is_empty());
    }

    #[test]
    fn list_installed_returns_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let yaml_a = "name: alpha\nversion: '1.0'\nentry_fn: collect\n";
        let yaml_b = "name: beta\nversion: '2.0'\nentry_fn: run\n";

        store_manifest(dir.path(), "beta", yaml_b).unwrap();
        store_manifest(dir.path(), "alpha", yaml_a).unwrap();

        let list = list_installed(dir.path());
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "alpha");
        assert_eq!(list[1].0, "beta");
    }

    #[test]
    fn remove_plugin_deletes_files() {
        let dir = tempfile::tempdir().unwrap();
        store_blob(dir.path(), "doomed", b"fake").unwrap();
        store_manifest(
            dir.path(),
            "doomed",
            "name: doomed\nversion: '1'\nentry_fn: r\n",
        )
        .unwrap();

        remove_plugin(dir.path(), "doomed").unwrap();
        assert!(!dir.path().join("doomed.wasm").exists());
        assert!(!dir.path().join("doomed.manifest.yml").exists());
    }

    #[test]
    fn remove_nonexistent_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(remove_plugin(dir.path(), "ghost").is_err());
    }

    #[test]
    fn empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let plugins = scan_plugins_dir(dir.path(), None);
        assert!(plugins.is_empty());
    }
}
