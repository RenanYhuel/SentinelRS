pub mod api;
pub mod sse;

use anyhow::Result;

use crate::output::input;
use crate::store;

pub use api::ApiClient;

pub fn build_client(server_flag: Option<&str>) -> Result<ApiClient> {
    let cfg = store::load().ok();
    let url = resolve_url(server_flag, cfg.as_ref())?;

    if let Some(ref cfg) = cfg {
        let token = &cfg.auth.jwt_token;
        if !token.is_empty() {
            return Ok(ApiClient::with_token(&url, token));
        }
    }

    Ok(ApiClient::new(&url))
}

fn resolve_url(flag: Option<&str>, cfg: Option<&store::CliConfig>) -> Result<String> {
    if let Some(url) = flag {
        return Ok(normalize(url));
    }

    if let Some(cfg) = cfg {
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
