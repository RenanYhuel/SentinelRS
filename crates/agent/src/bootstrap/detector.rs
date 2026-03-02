use std::path::Path;

pub fn needs_bootstrap(config_path: &Path) -> bool {
    !config_path.exists()
}

pub fn bootstrap_token_from_env() -> Option<String> {
    std::env::var("BOOTSTRAP_TOKEN")
        .ok()
        .filter(|v| !v.is_empty())
}

pub fn server_url_from_env() -> Option<String> {
    std::env::var("SERVER_URL").ok().filter(|v| !v.is_empty())
}
