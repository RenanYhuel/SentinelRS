use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CliConfig {
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub auth: AuthSection,
    #[serde(default)]
    pub defaults: DefaultsSection,
    #[serde(default)]
    pub docker: DockerSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_rest_url")]
    pub url: String,
    #[serde(default = "default_grpc_url")]
    pub grpc_url: String,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            url: default_rest_url(),
            grpc_url: default_grpc_url(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthSection {
    #[serde(default)]
    pub jwt_token: String,
    #[serde(default)]
    pub token_expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSection {
    #[serde(default = "default_output")]
    pub output_format: String,
    #[serde(default = "default_true")]
    pub color: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSection {
    #[serde(default)]
    pub compose_file: String,
    #[serde(default = "default_project")]
    pub project_name: String,
}

fn default_rest_url() -> String {
    "http://localhost:8080".into()
}

fn default_grpc_url() -> String {
    "http://localhost:50051".into()
}

fn default_output() -> String {
    "human".into()
}

fn default_true() -> bool {
    true
}

fn default_project() -> String {
    "sentinel".into()
}

impl Default for DefaultsSection {
    fn default() -> Self {
        Self {
            output_format: default_output(),
            color: true,
        }
    }
}

impl Default for DockerSection {
    fn default() -> Self {
        Self {
            compose_file: String::new(),
            project_name: default_project(),
        }
    }
}

impl CliConfig {
    pub fn server_url(&self) -> &str {
        &self.server.url
    }

    pub fn output(&self) -> &str {
        &self.defaults.output_format
    }

    pub fn set_dotted(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "server.url" => self.server.url = value.to_string(),
            "server.grpc_url" => self.server.grpc_url = value.to_string(),
            "auth.jwt_token" => self.auth.jwt_token = value.to_string(),
            "auth.token_expires_at" => self.auth.token_expires_at = value.to_string(),
            "defaults.output_format" => self.defaults.output_format = value.to_string(),
            "defaults.color" => {
                self.defaults.color = value.parse().map_err(|_| "expected true or false")?;
            }
            "docker.compose_file" => self.docker.compose_file = value.to_string(),
            "docker.project_name" => self.docker.project_name = value.to_string(),
            _ => return Err(format!("unknown key: {key}")),
        }
        Ok(())
    }

    pub fn masked_token(&self) -> String {
        let t = &self.auth.jwt_token;
        if t.len() <= 8 {
            "***".into()
        } else {
            format!("{}...{}", &t[..4], &t[t.len() - 4..])
        }
    }
}
