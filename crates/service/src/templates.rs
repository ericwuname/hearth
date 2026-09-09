// v8.0: Agent template system.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A named agent template (stores as a .toml file under the templates/ directory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub name: String,
    pub model: Option<String>,
    pub system_prompt: String,
    #[serde(default)]
    pub tools: Vec<String>,
    pub max_steps: Option<u64>,
    pub temperature: Option<f64>,
}

/// Scans a directory of .toml files and indexes them by name.
pub struct TemplateManager {
    templates: HashMap<String, AgentTemplate>,
}

impl TemplateManager {
    pub fn load(dir: &Path) -> std::io::Result<Self> {
        let mut templates = HashMap::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "toml") {
                    if let Ok(s) = std::fs::read_to_string(&p) {
                        if let Ok(t) = toml::from_str::<AgentTemplate>(&s) {
                            templates.insert(t.name.clone(), t);
                        }
                    }
                }
            }
        }
        Ok(Self { templates })
    }

    pub fn list(&self) -> Vec<&AgentTemplate> {
        self.templates.values().collect()
    }

    pub fn get(&self, name: &str) -> Option<&AgentTemplate> {
        self.templates.get(name)
    }
}
