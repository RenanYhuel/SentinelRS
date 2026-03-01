use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::path::Path;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_blob(blob: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(public_key) else {
        return false;
    };
    mac.update(blob);
    mac.verify_slice(signature).is_ok()
}

pub fn sign_blob(blob: &[u8], key: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("valid key length");
    mac.update(blob);
    mac.finalize().into_bytes().to_vec()
}

pub fn store_blob(
    plugins_dir: &Path,
    name: &str,
    blob: &[u8],
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(plugins_dir)?;
    let path = plugins_dir.join(format!("{name}.wasm"));
    std::fs::write(&path, blob)?;
    Ok(path)
}

pub fn store_manifest(
    plugins_dir: &Path,
    name: &str,
    manifest_yaml: &str,
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(plugins_dir)?;
    let path = plugins_dir.join(format!("{name}.manifest.yml"));
    std::fs::write(&path, manifest_yaml)?;
    Ok(path)
}

pub fn load_blob(plugins_dir: &Path, name: &str) -> std::io::Result<Vec<u8>> {
    let path = plugins_dir.join(format!("{name}.wasm"));
    std::fs::read(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let key = b"server-public-key";
        let blob = b"fake wasm module bytes";
        let sig = sign_blob(blob, key);
        assert!(verify_blob(blob, &sig, key));
    }

    #[test]
    fn tampered_blob_rejected() {
        let key = b"server-public-key";
        let blob = b"original";
        let sig = sign_blob(blob, key);
        assert!(!verify_blob(b"tampered", &sig, key));
    }

    #[test]
    fn wrong_key_rejected() {
        let blob = b"data";
        let sig = sign_blob(blob, b"key-a");
        assert!(!verify_blob(blob, &sig, b"key-b"));
    }

    #[test]
    fn store_and_load_blob() {
        let dir = tempfile::tempdir().unwrap();
        let blob = b"fake-wasm-content";
        let path = store_blob(dir.path(), "test_plugin", blob).unwrap();
        assert!(path.exists());

        let loaded = load_blob(dir.path(), "test_plugin").unwrap();
        assert_eq!(loaded, blob);
    }

    #[test]
    fn store_manifest_file() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "name: test\nversion: '1.0'\nentry_fn: run\n";
        let path = store_manifest(dir.path(), "test", yaml).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("name: test"));
    }
}
