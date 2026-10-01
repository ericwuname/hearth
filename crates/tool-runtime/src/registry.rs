// v10.5: Tool manifest system — 工具**清单目录**（catalog）。
//
// D-73（2026-10-01, traecode）**文档订正**：本模块此前自称 "secure tool
// installation" / "verification"，但实现**从不做任何校验**——`install()` 只把
// manifest 插进内存 map，既不校验摘要也不落地/执行任何产物。它只是 `GET
// /api/v1/tools/search`、`GET /api/v1/tool-registry`、`POST /api/v1/tools/install`
// 背后的**清单存储**（工具执行走 `ToolDispatcher` + `tools-builtin`，与本模块无关）。
// 依"声称≠实现"纪律（D-23/D-45/D-54 同族）订正文案，**零行为变更**。
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
    ///
    /// D-73：**当前全仓零消费者**（既无写入方也无读取方）——本模块不做任何摘要
    /// 校验。保留字段以兼容既有 manifest 文本（serde 忽略未知字段，删/留均不影响
    /// 解析）；**勿据此认为存在完整性校验**。
    pub sha256: String,
    /// Command to execute (e.g., "bash script.sh" or binary path).
    ///
    /// D-73：**当前全仓零消费者**——本模块只存清单、不执行工具（执行走
    /// `ToolDispatcher`）。同 `sha256`，勿据此认为本注册表会拉起进程。
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

/// Tool manifest registry with search / install.
///
/// D-73：原名 "Secure tool registry … and verification" 名不副实——**无校验**
/// （详见模块头）。
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

    /// Install a tool manifest into the registry.
    ///
    /// D-73：原文档称 "verifying SHA-256 if a file path is provided"——**不实**：
    /// 本方法**不做任何校验**，也**没有 file path 参数**，只把 manifest 插入内存
    /// map（同名校验仅为去重）。返回 `Ok(true)`=新登记、`Ok(false)`=已存在（跳过）。
    ///
    /// 注意：`POST /api/v1/tools/install` 直接消费本方法；因工具执行走
    /// `ToolDispatcher`（本注册表不执行任何东西），登记的 manifest 只影响
    /// `search`/`list` 的清单展示，不构成"安装即可执行"的路径。
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
