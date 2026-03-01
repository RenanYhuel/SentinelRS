use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub grpc_addr: SocketAddr,
    pub rest_addr: SocketAddr,
    pub jwt_secret: Vec<u8>,
    pub rate_limit_rps: u64,
    pub key_grace_period_ms: i64,
    pub replay_window_ms: i64,
    pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            grpc_addr: "0.0.0.0:50051".parse().unwrap(),
            rest_addr: "0.0.0.0:8080".parse().unwrap(),
            jwt_secret: b"change-me-in-production".to_vec(),
            rate_limit_rps: 100,
            key_grace_period_ms: 24 * 60 * 60 * 1000,
            replay_window_ms: 5 * 60 * 1000,
            tls: None,
        }
    }
}
