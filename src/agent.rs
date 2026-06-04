//! Bauer Agent HTTP client — optional AI integration.

pub struct AgentClient {
    pub base_url: String,
    pub timeout_secs: u64,
    pub enabled: bool,
}

impl AgentClient {
    pub fn new(base_url: String, timeout_secs: u64, enabled: bool) -> Self {
        Self { base_url, timeout_secs, enabled }
    }

    /// Check if the agent is reachable. Blocking.
    pub fn is_available(&self) -> bool {
        if !self.enabled {
            return false;
        }
        let url = format!("{}/health", self.base_url);
        reqwest::blocking::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Send page text and URL to the agent, return summary. Blocking.
    pub fn summarize(&self, url: &str, content: &str) -> Result<String, String> {
        if !self.enabled {
            return Err("Agent disabled".into());
        }
        let endpoint = format!("{}/summarize", self.base_url);
        let body = serde_json::json!({
            "url": url,
            "content": &content[..content.len().min(8000)],
            "mode": "summary"
        });
        let resp = reqwest::blocking::Client::new()
            .post(&endpoint)
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        Ok(json["summary"]
            .as_str()
            .unwrap_or("(sem resposta)")
            .to_string())
    }
}
