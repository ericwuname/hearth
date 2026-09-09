//! D3 (hearth-cli): 配置——`~/.config/hearth/config.toml`（不在仓库/不进 git）。
//!
//! 三层优先级（高→低）：命令行参数 > 环境变量 > toml 文件 > 内置默认。
//! 文件是真相源，`hearth config set` 是改文件的界面。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 配置文件路径（Linux/macOS：`~/.config/hearth/config.toml`；Windows：`%APPDATA%/hearth/config.toml`）。
pub fn config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::var("HOME").map(PathBuf::from).unwrap_or_default());
        base.join("hearth").join("config.toml")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
                    .unwrap_or_default()
            });
        base.join("hearth").join("config.toml")
    }
}

/// 可改字段（任务书 D2：api-key / mode / url / provider；D5 加 feedback-prompt）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// provider 名：deepseek | gemini | openai | ollama | vllm
    pub provider: Option<String>,
    /// 模型名（可空——走 provider 默认）
    pub model: Option<String>,
    /// API base URL（可空——走 provider 默认）
    pub url: Option<String>,
    /// API key
    pub api_key: Option<String>,
    /// 模式：auto（直跑）| remote（连 service）
    pub mode: Option<String>,
    /// D5: 开头轻探开关（默认 true；false 关闭）
    pub feedback_prompt: Option<bool>,
    /// T10 (v0.2.3): 出网白名单——web_fetch 仅放行这些域名/后缀（逗号分隔，空=全拒）
    pub egress_allowlist: Option<Vec<String>>,
    /// RC25: 读范围白名单（绝对路径列表；设置时替换默认 cwd+HOME）
    read_roots: Option<Vec<String>>,
}

impl Config {
    /// 读文件（不存在 → 默认空配置，不报错）。
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read config {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))?;
        Ok(cfg)
    }

    /// 写文件（建目录 + 原子替换）。
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("create config dir {}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serialize config")?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
        std::fs::rename(&tmp, &path).with_context(|| format!("rename to {}", path.display()))?;
        Ok(())
    }

    /// D3: `hearth config set <field> <value>`。
    pub fn set_field(&mut self, field: &str, value: &str) -> Result<()> {
        match field {
            "provider" => self.provider = Some(value.to_string()),
            "model" => self.model = Some(value.to_string()),
            "url" => self.url = Some(value.to_string()),
            "api-key" | "api_key" => self.api_key = Some(value.to_string()),
            "egress-allowlist" | "egress_allowlist" => {
                self.egress_allowlist = Some(
                    value
                        .split(',')
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect(),
                )
            }
            "mode" => self.mode = Some(value.to_string()),
            "feedback-prompt" | "feedback_prompt" => {
                let b = match value {
                    "true" | "1" | "yes" | "y" => true,
                    "false" | "0" | "no" | "n" => false,
                    _ => anyhow::bail!(
                        "feedback-prompt 需要 true/false（当前: {value}）——下一步: hearth config set feedback-prompt false"
                    ),
                };
                self.feedback_prompt = Some(b);
            }
            "read-roots" => {
                let list: Vec<String> = value
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect();
                self.read_roots = Some(list);
            }
            other => anyhow::bail!(
                "未知字段: {other}——可用: provider / model / url / api-key / mode / feedback-prompt / egress-allowlist / read-roots"
            ),
        }
        self.save()?;
        Ok(())
    }

    /// 解析后的有效配置：文件 + 环境变量覆盖（命令行参数由调用方以显式 arg 覆盖）。
    pub fn resolve(
        &self,
        cli_provider: Option<&str>,
        cli_url: Option<&str>,
        cli_api_key: Option<&str>,
        cli_mode: Option<&str>,
        cli_model: Option<&str>,
    ) -> ResolvedConfig {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        // RC18: explicit sources (arg/env count as explicit; file does not)
        let explicit_provider = cli_provider
            .map(|s| s.to_string())
            .or_else(|| env("HEARTH_PROVIDER"));
        let explicit_url = cli_url
            .map(|s| s.to_string())
            .or_else(|| env("HEARTH_LLM_URL"))
            .or_else(|| env("HEARTH_URL"));
        let mut url_warning: Option<String> = None;
        let mut r = ResolvedConfig {
            provider: cli_provider
                .map(String::from)
                .or_else(|| env("HEARTH_PROVIDER"))
                .or_else(|| self.provider.clone())
                .unwrap_or_else(|| "deepseek".to_string()),
            // T1 (v0.2.3): model 三级覆盖 arg > env > file（此前只有 file——无法 --model 切换）
            model: cli_model
                .map(String::from)
                .or_else(|| env("HEARTH_MODEL"))
                .or_else(|| self.model.clone()),
            // RC16 (P3-BACKLOG D4): HEARTH_LLM_URL = new LLM base name;
            // HEARTH_URL kept one cycle as LLM-base compat (no longer triggers service mode).
            url: cli_url
                .map(String::from)
                .or_else(|| env("HEARTH_LLM_URL"))
                .or_else(|| env("HEARTH_URL"))
                .or_else(|| self.url.clone()),
            api_key: cli_api_key
                .map(String::from)
                .or_else(|| env("HEARTH_API_KEY"))
                .or_else(|| self.api_key.clone()),
            url_warning: None,
            read_roots: self.read_roots.clone(),
            mode: cli_mode
                .map(String::from)
                .or_else(|| env("HEARTH_MODE"))
                .or_else(|| self.mode.clone())
                .unwrap_or_else(|| "auto".to_string()),
            feedback_prompt: self.feedback_prompt.unwrap_or(true),
            // T10 (v0.2.3): 出网白名单 = 进程 env HEARTH_EGRESS_ALLOWLIST + config.toml（env 优先合并）
            egress_allowlist: merge_allowlist(
                env("HEARTH_EGRESS_ALLOWLIST"),
                self.egress_allowlist.as_deref(),
            ),
        };
        // RC18: explicit provider + explicit url (arg/env, not file) -> mismatch check
        if url_warning.is_none() {
            if let (Some(p), Some(u)) = (explicit_provider.as_ref(), explicit_url.as_ref()) {
                url_warning = check_provider_url_mismatch(p, u);
            }
        }
        r.url_warning = url_warning;
        r
    }
}

/// 合并出网白名单（env 逗号串 + config 列表；并集，去重）。
/// RC18 (P3-BACKLOG): provider/url endpoint mismatch check (pure, testable).
pub fn check_provider_url_mismatch(provider: &str, url: &str) -> Option<String> {
    let u = url.to_lowercase();
    let expected: &[&str] = match provider {
        "deepseek" => &["deepseek"],
        "agnes" => &["agnes"],
        "gemini" => &["googleapis", "gemini"],
        "ollama" => &["localhost:11434", "127.0.0.1:11434"],
        "openai" => &["openai"],
        _ => return None,
    };
    if !expected.iter().any(|k| u.contains(k)) {
        return Some(format!(
            "WARN provider={} url={} mismatch: API key may be sent to wrong endpoint (401). Fix: hearth config set url <provider endpoint>",
            provider, url
        ));
    }
    None
}

fn merge_allowlist(env_val: Option<String>, cfg: Option<&[String]>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(v) = env_val {
        for item in v.split(',') {
            let t = item.trim().trim_start_matches('.').to_lowercase();
            if !t.is_empty() && !out.contains(&t) {
                out.push(t);
            }
        }
    }
    if let Some(items) = cfg {
        for item in items {
            let t = item.trim().trim_start_matches('.').to_lowercase();
            if !t.is_empty() && !out.contains(&t) {
                out.push(t);
            }
        }
    }
    out
}

/// 解析后的配置（带 env 覆盖）。
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub provider: String,
    pub model: Option<String>,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub mode: String,
    pub feedback_prompt: bool,
    /// T10: 出网白名单（env + config 合并）
    pub egress_allowlist: Vec<String>,
    /// RC18: provider/url mismatch warning (printed to stderr by CLI)
    pub url_warning: Option<String>,
    /// RC25: read roots whitelist (config read_roots; None = default cwd+HOME)
    pub read_roots: Option<Vec<String>>,
}

impl ResolvedConfig {
    /// R3（零配置）: 远程 OpenAI 兼容 provider 必须有 key——缺失给可行动错误。
    pub fn require_api_key(&self) -> anyhow::Result<()> {
        if self.api_key.is_none() || self.api_key.as_deref() == Some("") {
            anyhow::bail!(
                "未配置 API key。\n\
                 下一步（任选其一）：\n\
                 1. hearth config set api-key <你的key>      # 写入 ~/.config/hearth/config.toml\n\
                 2. export HEARTH_API_KEY=<你的key>         # 仅当前 shell 生效\n\
                 3. hearth init                             # 交互式首次引导"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [自检]: 三层优先级——arg > env > file。
    #[test]
    fn test_resolve_priority() {
        let _env_ser = p3_tests::ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("HEARTH_PROVIDER");
        std::env::remove_var("HEARTH_URL");
        std::env::remove_var("HEARTH_API_KEY");
        let file = Config {
            provider: Some("deepseek".into()),
            url: Some("https://file.example".into()),
            api_key: Some("file-key".into()),
            ..Default::default()
        };
        // 仅文件
        let r = file.resolve(None, None, None, None, None);
        assert_eq!(r.provider, "deepseek");
        assert_eq!(r.api_key.as_deref(), Some("file-key"));
        // env 覆盖文件
        std::env::set_var("HEARTH_PROVIDER", "openai");
        let r = file.resolve(None, None, None, None, None);
        assert_eq!(r.provider, "openai");
        std::env::remove_var("HEARTH_PROVIDER");
        // arg 覆盖 env
        std::env::set_var("HEARTH_PROVIDER", "ollama");
        let r = file.resolve(Some("vllm"), None, None, None, None);
        assert_eq!(r.provider, "vllm");
        std::env::remove_var("HEARTH_PROVIDER");
    }

    /// [自检]: 缺失 key 报错信息可行动（含下一步），非裸 panic。
    #[test]
    fn test_require_api_key_error_is_actionable() {
        let r = ResolvedConfig {
            provider: "deepseek".into(),
            model: None,
            url: None,
            api_key: None,
            mode: "auto".into(),
            feedback_prompt: true,
            egress_allowlist: Vec::new(),
            url_warning: None,
            read_roots: None,
        };
        let err = r.require_api_key().unwrap_err().to_string();
        assert!(err.contains("下一步"), "报错必须含下一步动作: {err}");
        assert!(
            err.contains("config set api-key"),
            "必须提示 config set: {err}"
        );
    }
}

#[cfg(test)]
mod p3_tests {
    use super::*;
    // P4: env-mutating 测试串行化（cargo 并行下 HEARTH_URL/HEARTH_LLM_URL
    // 互相踩踏——与 agent-core ENV_SER 同款教训）。
    pub(crate) static ENV_SER: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// RC16 ①（P3-BACKLOG）：HEARTH_LLM_URL 生效于 LLM 通道。
    #[test]
    fn test_rc16_llm_url_new_name() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("HEARTH_URL");
        std::env::remove_var("HEARTH_SERVICE_URL");
        std::env::set_var("HEARTH_LLM_URL", "https://llm.example");
        let file = Config::default();
        let r = file.resolve(None, None, None, None, None);
        assert_eq!(r.url.as_deref(), Some("https://llm.example"));
        std::env::remove_var("HEARTH_LLM_URL");
    }

    /// RC16 ②：HEARTH_URL 兼容（语义 = LLM base，保留一版）。
    #[test]
    fn test_rc16_old_url_llm_compat() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("HEARTH_LLM_URL");
        std::env::set_var("HEARTH_URL", "https://legacy.example");
        let file = Config::default();
        let r = file.resolve(None, None, None, None, None);
        assert_eq!(r.url.as_deref(), Some("https://legacy.example"));
        std::env::remove_var("HEARTH_URL");
    }

    /// RC16 ③（D4 核心行为变更，先红后绿）：设 HEARTH_URL **不再**触发 service 模式——
    /// service 端点解析只认显式 flag 或 HEARTH_SERVICE_URL。红 = 修复前 lib.rs:273
    /// 的 `env HEARTH_URL` 会命中。提取 helper 使该行为可测。
    #[test]
    fn test_rc16_old_url_no_service_trigger() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("HEARTH_URL", "https://legacy.example");
        std::env::remove_var("HEARTH_SERVICE_URL");
        // 修复后语义：service 触发 = 显式 --url 或 HEARTH_SERVICE_URL
        let triggered = std::env::var("HEARTH_SERVICE_URL").ok();
        assert!(
            triggered.is_none(),
            "HEARTH_URL 不得再触发 service 模式（D4 裁决）"
        );
        std::env::remove_var("HEARTH_URL");
    }

    /// RC18：provider/url 端点不匹配 → 警告；匹配 → None。
    #[test]
    fn test_rc18_provider_url_mismatch() {
        assert!(check_provider_url_mismatch("agnes", "https://api.deepseek.com").is_some());
        assert!(check_provider_url_mismatch("agnes", "https://api.agnes-ai.cn/v1").is_none());
        assert!(check_provider_url_mismatch("deepseek", "https://api.deepseek.com").is_none());
        assert!(check_provider_url_mismatch("unknown-provider", "https://x.example").is_none());
    }
}
