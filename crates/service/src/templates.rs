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
    /// 从目录加载模板清单（每个 `*.toml` 一份 `AgentTemplate`）。
    ///
    /// D-111（2026-10-02, traecode）：单个模板文件**读取失败 / TOML 解析失败**
    /// 此前被两层 `if let Ok` **静默丢弃**——用户看到的现象是"我的模板不见了"，
    /// 却没有任何提示（同族：D-60/D-63 的"不静默降级"纪律）。现逐项 **warn 留痕**，
    /// 且失败**不阻断**其余模板加载（best-effort 语义不变）。
    pub fn load(dir: &Path) -> std::io::Result<Self> {
        let mut templates = HashMap::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "toml") {
                    let s = match std::fs::read_to_string(&p) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!(path = %p.display(), "模板文件读取失败，已跳过: {e}");
                            continue;
                        }
                    };
                    match toml::from_str::<AgentTemplate>(&s) {
                        Ok(t) => {
                            templates.insert(t.name.clone(), t);
                        }
                        Err(e) => {
                            tracing::warn!(path = %p.display(), "模板 TOML 解析失败，已跳过: {e}");
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
