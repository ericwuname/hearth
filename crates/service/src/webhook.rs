// v8.0: Webhook notification system.
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// A registered webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub events: Vec<String>,
}

/// In-memory webhook registry (per-user keyed by user_id, for future per-user isolation).
pub struct WebhookManager {
    hooks: RwLock<Vec<WebhookConfig>>,
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookManager {
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// Register a new webhook.
    pub fn register(&self, cfg: WebhookConfig) {
        self.hooks.write().unwrap().push(cfg);
    }

    /// Fire all webhooks that match the given event name.
    /// Runs fire-and-forget via curl subprocess (no extra dep needed).
    pub async fn fire(event: &str, payload: &serde_json::Value, hooks: &[WebhookConfig]) {
        for h in hooks {
            if h.events.iter().any(|e| e == event) {
                let url = h.url.clone();
                let body = payload.to_string();
                tokio::spawn(async move {
                    let cmd = tokio::process::Command::new("curl")
                        .args([
                            "-s",
                            "-X",
                            "POST",
                            "-H",
                            "Content-Type: application/json",
                            "-d",
                            &body,
                            &url,
                        ])
                        .output();
                    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), cmd).await;
                });
            }
        }
    }

    /// v10.4: Fire matching webhooks from the registry.
    pub async fn fire_event(&self, event: &str, payload: &serde_json::Value) {
        let hooks = self.hooks.read().unwrap().clone();
        if !hooks.is_empty() {
            Self::fire(event, payload, &hooks).await;
        }
    }

    pub fn list(&self) -> Vec<WebhookConfig> {
        self.hooks.read().unwrap().clone()
    }
}
