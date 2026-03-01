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

impl ServerConfig {
    pub fn from_env_and_args() -> Self {
        let mut config = Self::default();
        let args = CliArgs::parse();

        if let Some(addr) = args.grpc_addr {
            config.grpc_addr = addr;
        } else if let Some(addr) = env_socket_addr("GRPC_ADDR") {
            config.grpc_addr = addr;
        } else if let Some(port) = env_port("GRPC_PORT") {
            config.grpc_addr.set_port(port);
        }

        if let Some(addr) = args.rest_addr {
            config.rest_addr = addr;
        } else if let Some(addr) = env_socket_addr("REST_ADDR") {
            config.rest_addr = addr;
        } else if let Some(port) = env_port("REST_PORT") {
            config.rest_addr.set_port(port);
        }

        if let Some(ref secret) = args.jwt_secret {
            config.jwt_secret = secret.as_bytes().to_vec();
        } else if let Ok(val) = std::env::var("JWT_SECRET") {
            config.jwt_secret = val.into_bytes();
        }

        if let Some(cert) = args.tls_cert {
            config.tls = Some(TlsConfig {
                cert_path: cert,
                key_path: args.tls_key.unwrap_or_default(),
                ca_path: args.tls_ca,
            });
        }

        config
    }
}

fn env_socket_addr(key: &str) -> Option<SocketAddr> {
    std::env::var(key).ok()?.parse().ok()
}

fn env_port(key: &str) -> Option<u16> {
    std::env::var(key).ok()?.parse().ok()
}

struct CliArgs {
    grpc_addr: Option<SocketAddr>,
    rest_addr: Option<SocketAddr>,
    jwt_secret: Option<String>,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
    tls_ca: Option<PathBuf>,
}

impl CliArgs {
    fn parse() -> Self {
        let mut result = Self {
            grpc_addr: None,
            rest_addr: None,
            jwt_secret: None,
            tls_cert: None,
            tls_key: None,
            tls_ca: None,
        };

        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--version" | "-V" => {
                    println!("sentinel_server {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--grpc-addr" => {
                    result.grpc_addr = args.next().and_then(|v| v.parse().ok());
                }
                "--grpc-port" => {
                    if let Some(port) = args.next().and_then(|v| v.parse::<u16>().ok()) {
                        let mut addr: SocketAddr = "0.0.0.0:50051".parse().unwrap();
                        addr.set_port(port);
                        result.grpc_addr = Some(addr);
                    }
                }
                "--rest-addr" => {
                    result.rest_addr = args.next().and_then(|v| v.parse().ok());
                }
                "--rest-port" => {
                    if let Some(port) = args.next().and_then(|v| v.parse::<u16>().ok()) {
                        let mut addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
                        addr.set_port(port);
                        result.rest_addr = Some(addr);
                    }
                }
                "--jwt-secret" => {
                    result.jwt_secret = args.next();
                }
                "--tls-cert" => {
                    result.tls_cert = args.next().map(PathBuf::from);
                }
                "--tls-key" => {
                    result.tls_key = args.next().map(PathBuf::from);
                }
                "--tls-ca" => {
                    result.tls_ca = args.next().map(PathBuf::from);
                }
                other => {
                    eprintln!("error: unknown argument '{other}'");
                    eprintln!("run with --help for usage");
                    std::process::exit(1);
                }
            }
        }

        result
    }
}

fn print_help() {
    println!("Usage: sentinel_server [OPTIONS]\n");
    println!("Options:");
    println!("      --grpc-addr <ADDR>   gRPC listen address  (default: 0.0.0.0:50051)");
    println!("      --grpc-port <PORT>   gRPC listen port     (default: 50051)");
    println!("      --rest-addr <ADDR>   REST listen address  (default: 0.0.0.0:8080)");
    println!("      --rest-port <PORT>   REST listen port     (default: 8080)");
    println!("      --jwt-secret <KEY>   JWT signing secret");
    println!("      --tls-cert <PATH>    TLS certificate path");
    println!("      --tls-key <PATH>     TLS private key path");
    println!("      --tls-ca <PATH>      TLS CA certificate path");
    println!("  -V, --version            Print version");
    println!("  -h, --help               Print help");
    println!("\nEnvironment variables:");
    println!("  GRPC_ADDR    Full gRPC listen address (e.g. 0.0.0.0:50051)");
    println!("  GRPC_PORT    gRPC port only");
    println!("  REST_ADDR    Full REST listen address (e.g. 0.0.0.0:8080)");
    println!("  REST_PORT    REST port only");
    println!("  JWT_SECRET   JWT signing secret");
    println!("\nPrecedence: CLI flags > environment variables > defaults");
}
