use std::fs;

use crate::config::TlsConfig;

pub struct TlsIdentity {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub ca_pem: Option<Vec<u8>>,
}

impl TlsIdentity {
    pub fn load(config: &TlsConfig) -> Result<Self, TlsLoadError> {
        let cert_pem = fs::read(&config.cert_path)
            .map_err(|e| TlsLoadError(format!("cert: {e}")))?;
        let key_pem = fs::read(&config.key_path)
            .map_err(|e| TlsLoadError(format!("key: {e}")))?;
        let ca_pem = config
            .ca_path
            .as_ref()
            .map(|p| fs::read(p).map_err(|e| TlsLoadError(format!("ca: {e}"))))
            .transpose()?;
        Ok(Self {
            cert_pem,
            key_pem,
            ca_pem,
        })
    }

    pub fn tonic_server_tls(&self) -> Result<tonic::transport::ServerTlsConfig, TlsLoadError> {
        let identity =
            tonic::transport::Identity::from_pem(&self.cert_pem, &self.key_pem);
        let mut tls = tonic::transport::ServerTlsConfig::new().identity(identity);
        if let Some(ca) = &self.ca_pem {
            let ca_cert = tonic::transport::Certificate::from_pem(ca);
            tls = tls.client_ca_root(ca_cert);
        }
        Ok(tls)
    }
}

#[derive(Debug)]
pub struct TlsLoadError(pub String);

impl std::fmt::Display for TlsLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TLS: {}", self.0)
    }
}

impl std::error::Error for TlsLoadError {}
