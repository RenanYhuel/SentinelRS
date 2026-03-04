pub mod api;
pub mod sse;

use anyhow::Result;

use crate::output::input;
use crate::store;

pub use api::ApiClient;

pub fn build_client(server_flag: Option<&str>) -> Result<ApiClient> {
    let url = resolve_url(server_flag)?;
    Ok(ApiClient::new(&url))
}

fn resolve_url(flag: Option<&str>) -> Result<String> {
    if let Some(url) = flag {
        return Ok(normalize(url));
    }

    if let Ok(cfg) = store::load() {
        return Ok(normalize(cfg.server_url()));
    }

    let url = input::text("Server URL", "http://localhost:8080")?;
    Ok(normalize(&url))
}

pub(crate) fn normalize(url: &str) -> String {
    let mut u = url.to_string();
    if u.starts_with("grpc://") {
        u = u.replace("grpc://", "http://");
    }
    u.trim_end_matches('/').to_string()
}
