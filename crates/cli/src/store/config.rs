use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub server_url: String,
    #[serde(default = "default_output")]
    pub output: String,
}

fn default_output() -> String {
    "human".into()
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".into(),
            output: default_output(),
        }
    }
}
