use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub grpc_addr: SocketAddr,
    pub rest_addr: SocketAddr,
    pub jwt_secret: Vec<u8>,
    pub rate_limit_rps: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            grpc_addr: "0.0.0.0:50051".parse().unwrap(),
            rest_addr: "0.0.0.0:8080".parse().unwrap(),
            jwt_secret: b"change-me-in-production".to_vec(),
            rate_limit_rps: 100,
        }
    }
}
