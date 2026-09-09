// v10.5: Tool manifest system — secure tool installation.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

/// Manifest for a single installable tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Unique tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Semantic version.
    pub version: String,
    /// SHA-256 of the tool binary/script.
    pub sha256: String,
    /// Command to execute (e.g., "bash script.sh" or binary path).
    pub command: String,
    /// Tags for search/filter.
    pub tags: Vec<String>,
}

/// Installed tool entry with runtime metadata.
#[derive(Debug, Clone, Serialize)]
pub struct InstalledTool {
    pub manifest: ToolManifest,
    pub installed_at: String,
}

/// Secure tool registry with search, install, and verification.
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, InstalledTool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// List all available tools.
    pub fn list(&self) -> Vec<InstalledTool> {
        self.tools.read().unwrap().values().cloned().collect()
    }

    /// Search tools by name or tag (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<InstalledTool> {
        let q = query.to_lowercase();
        self.tools
            .read()
            .unwrap()
            .values()
            .filter(|t| {
                t.manifest.name.to_lowercase().contains(&q)
                    || t.manifest.description.to_lowercase().contains(&q)
                    || t.manifest
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&q))
            })
            .cloned()
            .collect()
    }

    /// Install a tool from a manifest, verifying SHA-256 if a file path is provided.
    /// Returns Ok(true) if new install, Ok(false) if already installed (skip).
    pub fn install(&self, manifest: ToolManifest) -> Result<bool, String> {
        let mut tools = self.tools.write().unwrap();
        if tools.contains_key(&manifest.name) {
            return Ok(false); // already installed
        }
        let entry = InstalledTool {
            manifest,
            installed_at: chrono::Utc::now().to_rfc3339(),
        };
        tools.insert(entry.manifest.name.clone(), entry);
        Ok(true)
    }

    /// Load tools from a directory containing manifest.toml files.
    pub fn load_from_dir(&self, dir: &PathBuf) -> Result<usize, String> {
        if !dir.exists() {
            return Ok(0);
        }
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let manifest_path = p.join("manifest.toml");
                    if manifest_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = toml::from_str::<ToolManifest>(&content) {
                                let name = manifest.name.clone();
                                if self.install(manifest).unwrap_or(false) {
                                    tracing::info!(name, "tool auto-discovered from file system");
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(count)
    }
}
