use anyhow::{Context, Result};
use reqwest::StatusCode;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

pub struct ApiClient {
    base_url: String,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub fn with_token(base_url: &str, token: &str) -> Self {
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
            headers.insert(AUTHORIZATION, val);
        }
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let resp = self
            .http
            .get(self.url(path))
            .send()
            .await
            .context("request failed")?
            .error_for_status()
            .context("server returned error")?;
        resp.json().await.context("invalid JSON response")
    }

    pub async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let resp = self
            .http
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .context("request failed")?
            .error_for_status()
            .context("server returned error")?;
        resp.json().await.context("invalid JSON response")
    }

    pub async fn put_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let resp = self
            .http
            .put(self.url(path))
            .json(body)
            .send()
            .await
            .context("request failed")?
            .error_for_status()
            .context("server returned error")?;
        resp.json().await.context("invalid JSON response")
    }

    pub async fn delete_path(&self, path: &str) -> Result<StatusCode> {
        let resp = self
            .http
            .delete(self.url(path))
            .send()
            .await
            .context("request failed")?;
        Ok(resp.status())
    }

    pub async fn post_empty(&self, path: &str) -> Result<serde_json::Value> {
        let resp = self
            .http
            .post(self.url(path))
            .send()
            .await
            .context("request failed")?
            .error_for_status()
            .context("server returned error")?;
        resp.json().await.context("invalid JSON response")
    }

    pub async fn get_text(&self, path: &str) -> Result<String> {
        let resp = self
            .http
            .get(self.url(path))
            .send()
            .await
            .context("request failed")?
            .error_for_status()
            .context("server returned error")?;
        resp.text().await.context("failed to read response body")
    }

    pub async fn health_ok(&self) -> bool {
        self.http
            .get(self.url("/healthz"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub fn streaming_client(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn streaming_url(&self, path: &str) -> String {
        self.url(path)
    }
}
