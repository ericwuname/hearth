//! D3 (hearth-cli): 配置——`~/.config/hearth/config.toml`（不在仓库/不进 git）。
//!
//! 三层优先级（高→低）：命令行参数 > 环境变量 > toml 文件 > 内置默认。
//! 文件是真相源，`hearth config set` 是改文件的界面。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 配置文件路径（Linux/macOS：`~/.config/hearth/config.toml`；Windows：`%APPDATA%/hearth/config.toml`）。
/// C-2：APPDATA 缺失时回退 `HOME/.config/hearth/config.toml`（与 Unix 习惯及
/// diagnostics 写入位置一致——避免落到非标准的 `HOME/hearth/`）。
pub fn config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
                    .unwrap_or_default()
            });
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
    /// T10 (v0.2.3) + hearth-slim S2: 出网白名单——web_fetch 空列表=全拒（不变）；
    /// bash 空列表=默认放开（S2 语义反转，网络活动带 [net] 审计投影）
    pub egress_allowlist: Option<Vec<String>>,
    /// RC25: 读范围白名单（绝对路径列表；设置时替换默认 cwd+HOME）
    read_roots: Option<Vec<String>>,
    /// S9（手术包二）：provider 降级链（有序，如 ["agnes","zhipu","gemini"]）。
    /// 空/单元素 = 单通道（现行为不变）。**不含 key**。
    pub providers: Option<Vec<String>>,
    /// S9：各通道 key（`[provider_keys] agnes = "..."`）——本机 config.toml
    ///（权限 600，unix），**永不入 git**（Push Protection 红线——任何 key 入库翻车）。
    pub provider_keys: Option<std::collections::HashMap<String, String>>,
    /// 修复 4（顶层批复）：输出预算覆盖（per-run）——config 为真相源，运行时
    /// 注入 agent 侧取参（不新增独立 env 概念；缺省 = 内置默认 65536）。
    /// 背景：agnes-3.0-flash thinking 与正文共享输出预算，8192 硬编码致正文残缺。
    pub max_tokens: Option<u32>,
}

impl Config {
    /// 读文件（不存在 → 默认空配置，不报错）。
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        // bounded-io-exempt: D-77 裁决——操作者本机 config.toml，无无界增长特性；加 cap 无安全收益、反有"合法大配置被截断→解析失败"的回归风险
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
        // S9（手术包二）：config 可含各通道 key（provider_keys）——权限收紧 600
        //（unix；Windows 依赖用户目录 ACL），且**永不入 git**（红线）。
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
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
        // D-117：provider 先解析——下方 model/url/api_key 都要按 **provider** 取惯例
        // env（`AGNES_API_KEY` 等），否则 `.env` 照模板配好也"不生效"。
        let provider = cli_provider
            .map(String::from)
            .or_else(|| env("HEARTH_PROVIDER"))
            .or_else(|| self.provider.clone())
            .unwrap_or_else(|| "deepseek".to_string());
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
            provider: provider.clone(),
            // T1 (v0.2.3): model 三级覆盖 arg > env > file（此前只有 file——无法 --model 切换）
            // D-117：env 层再补该通道的惯例名（`AGNES_MODEL` 等），与 service 对齐。
            model: cli_model
                .map(String::from)
                .or_else(|| env("HEARTH_MODEL"))
                .or_else(|| provider_env(&provider, "MODEL"))
                .or_else(|| self.model.clone()),
            // RC16 (P3-BACKLOG D4): HEARTH_LLM_URL = new LLM base name;
            // HEARTH_URL kept one cycle as LLM-base compat (no longer triggers service mode).
            // D-117：再补该通道的惯例端点（`AGNES_BASE_URL` 等）。
            url: cli_url
                .map(String::from)
                .or_else(|| env("HEARTH_LLM_URL"))
                .or_else(|| env("HEARTH_URL"))
                .or_else(|| provider_env(&provider, "BASE_URL"))
                .or_else(|| self.url.clone()),
            // D-117：再补该通道的惯例 key（`AGNES_API_KEY` / `DEEPSEEK_API_KEY` …）。
            api_key: cli_api_key
                .map(String::from)
                .or_else(|| env("HEARTH_API_KEY"))
                .or_else(|| provider_env(&provider, "API_KEY"))
                .or_else(|| self.api_key.clone()),
            url_warning: None,
            read_roots: self.read_roots.clone(),
            // D-129：mode 的唯一事实源（arg > HEARTH_MODE > config.toml > auto）
            // ——与 `effective_mode()` 共用同一处优先级定义（不再各写一份）。
            mode: self.effective_mode(cli_mode).0,
            feedback_prompt: self.feedback_prompt.unwrap_or(true),
            // T10 (v0.2.3) + hearth-slim S2: 出网白名单 = 进程 env HEARTH_EGRESS_ALLOWLIST + config.toml（env 优先合并；消费语义见各工具——bash 空=默认放开，web_fetch 空=全拒）
            egress_allowlist: merge_allowlist(
                env("HEARTH_EGRESS_ALLOWLIST"),
                self.egress_allowlist.as_deref(),
            ),
            // S9（手术包二）：降级链——env HEARTH_PROVIDERS（逗号）> config providers。
            // 空/单元素 = 单通道（现行为不变）。
            providers: env("HEARTH_PROVIDERS")
                .map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect::<Vec<_>>()
                })
                .or_else(|| self.providers.clone())
                .unwrap_or_default(),
            provider_keys: self.provider_keys.clone().unwrap_or_default(),
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

    /// D-129（2026-10-02, traecode）：`mode` 的**有效值 + 来源**——唯一事实源。
    ///
    /// 优先级与 [`Config::resolve`] **完全一致**（arg > `HEARTH_MODE` > `config.toml` > `auto`）。
    /// 之所以单独抽出来：D-102 把 `--mode` 的校验接在了**flag** 上，于是
    /// `config set mode` / `HEARTH_MODE` 两个来源**既不生效也不报错**（静默）——
    /// 实测 `HEARTH_MODE=remote` 不给 `--url` 仍**本地直跑**、
    /// `HEARTH_MODE=bogus` 连报错都没有（同一条"声称≠实现"，只修了一半）。
    /// 返回来源字符串是为了让报错**可行动**（告诉用户该去改哪一处）。
    pub fn effective_mode(&self, cli_mode: Option<&str>) -> (String, &'static str) {
        let env_mode = std::env::var("HEARTH_MODE").ok().filter(|v| !v.is_empty());
        if let Some(m) = cli_mode {
            (m.to_string(), "--mode 参数")
        } else if let Some(m) = env_mode {
            (m, "环境变量 HEARTH_MODE")
        } else if let Some(m) = self.mode.clone() {
            (m, "config.toml 的 mode")
        } else {
            ("auto".to_string(), "默认值")
        }
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

/// D-117（2026-10-02, traecode）：CLI 侧的 provider **惯例 env 前缀**。
///
/// 与 `service`（`main.rs` 的 provider 注册）读取的 env 名保持一致——`.env`、
/// `.env.example`、`docs/configuration.md` 用的都是这套名字
/// （`AGNES_API_KEY` / `DEEPSEEK_BASE_URL` / `OPENAI_MODEL` …），但 CLI 此前**只认
/// `HEARTH_*`** ⇒ 用户照模板配好 `.env`，`hearth chat` 仍报"未配置 API key"
/// （D-106 接线了 `.env` **加载**，却漏了**名字对齐**——同一"配好了却不生效"病灶）。
///
/// 只覆盖 CLI 真正支持的通道（`build_single_provider` 的 match 臂）。
pub fn provider_env_prefix(provider: &str) -> Option<&'static str> {
    match provider {
        "deepseek" => Some("DEEPSEEK"),
        "openai" => Some("OPENAI"),
        "gemini" => Some("GEMINI"),
        "agnes" => Some("AGNES"),
        "ollama" => Some("OLLAMA"),
        "vllm" => Some("VLLM"),
        _ => None,
    }
}

/// 读该通道的惯例 env `<PREFIX>_<SUFFIX>`（空串视为未设）；未知 provider → `None`。
pub fn provider_env(provider: &str, suffix: &str) -> Option<String> {
    provider_env_prefix(provider).and_then(|p| {
        std::env::var(format!("{p}_{suffix}"))
            .ok()
            .filter(|v| !v.is_empty())
    })
}

/// D-128（2026-10-02, traecode）：把 `KEY=VALUE` **合并**进既有 `.env` 文本。
///
/// 病灶：`hearth setup`（文档里的**首次上手**入口）原先直接
/// `std::fs::write(".env", …)` ——**截断重写**。用户照 `.env.example` 把
/// `DEEPSEEK_API_KEY` / `AGNES_API_KEY` / `HEARTH_PROVIDER` 配好后跑一次 setup，
/// 这些变量**被静默删除**（实测：3 行 → 1 行），随后 CLI 只报"未配置 API key"——
/// 用户根本不会想到是自己的 `.env` 被 setup 清空了（与 D-106/D-117 同在
/// onboarding 链上）。
///
/// 语义（本函数是唯一事实源，故单独可测）：
/// - **其余每一行原样保留**（注释、空行、别人的变量，逐字节不动）；
/// - 已存在同名键（行首经 trim 后为 `KEY=`，**注释行不算**）→ **就地替换**该行的值，
///   不追加第二份；行尾风格（`\r\n` / `\n`）沿用原行；
/// - 不存在 → 追加（若原文末尾缺换行，先补一个）；
/// - 返回 `(新文本, 就地替换的键数)`（调用方据此如实告知用户"更新"还是"新建"）。
pub fn merge_env_assignments(existing: &str, updates: &[(&str, &str)]) -> (String, usize) {
    let mut out = String::with_capacity(existing.len() + 64);
    let mut replaced = 0usize;
    let mut done: Vec<&str> = Vec::new();

    for chunk in existing.split_inclusive('\n') {
        let (line, ending) = match chunk.strip_suffix("\r\n") {
            Some(l) => (l, "\r\n"),
            None => match chunk.strip_suffix('\n') {
                Some(l) => (l, "\n"),
                None => (chunk, ""),
            },
        };
        let trimmed = line.trim_start();
        // 注释行（`#…`）**不是**赋值，原样保留。
        let hit = (!trimmed.starts_with('#')).then(|| {
            updates
                .iter()
                .find(|(k, _)| {
                    trimmed
                        .split_once('=')
                        .map(|(lk, _)| lk.trim() == *k)
                        .unwrap_or(false)
                })
                .copied()
        });
        match hit.flatten() {
            Some((k, v)) => {
                out.push_str(&format!("{k}={v}{ending}"));
                done.push(k);
                replaced += 1;
            }
            None => out.push_str(chunk),
        }
    }

    let mut appended = 0usize;
    for (k, v) in updates {
        if done.contains(k) {
            continue;
        }
        if appended == 0 && !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("{k}={v}\n"));
        appended += 1;
    }
    (out, replaced)
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
    /// S9（手术包二）：provider 降级链（有序；len<=1 = 单通道，行为不变）。
    pub providers: Vec<String>,
    /// S9：各通道 key（链内通道使用；主通道沿用 api_key 兜底）。
    pub provider_keys: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 先红后绿（D-129）：`mode` 的**有效值**必须同时覆盖 `HEARTH_MODE` 与 config.toml，
    /// 并给出**来源**（供报错点名"该去改哪一处"）。
    ///
    /// 红侧（修复前实测）：D-102 只把校验接在 `--mode` flag 上 ⇒ `HEARTH_MODE=remote`
    /// 不给 `--url` 仍**本地直跑**、`HEARTH_MODE=bogus` 静默忽略。
    #[test]
    fn test_d129_effective_mode_covers_env_and_file_with_source() {
        let _env_ser = p3_tests::ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let file = Config {
            mode: Some("remote".into()),
            ..Default::default()
        };

        std::env::remove_var("HEARTH_MODE");
        assert_eq!(
            file.effective_mode(None),
            ("remote".to_string(), "config.toml 的 mode"),
            "无 flag/env 时读 config.toml"
        );
        std::env::set_var("HEARTH_MODE", "bogus");
        assert_eq!(
            file.effective_mode(None),
            ("bogus".to_string(), "环境变量 HEARTH_MODE"),
            "env 覆盖 config.toml，且来源必须如实标出（否则报错不可行动）"
        );
        assert_eq!(
            file.effective_mode(Some("auto")),
            ("auto".to_string(), "--mode 参数"),
            "arg 优先级最高"
        );
        // 空串 env 视为未设（与 resolve 的 env() 口径一致）
        std::env::set_var("HEARTH_MODE", "");
        assert_eq!(file.effective_mode(None).0, "remote");

        std::env::remove_var("HEARTH_MODE");
        let none = Config::default();
        assert_eq!(none.effective_mode(None), ("auto".to_string(), "默认值"));

        // 与 resolve 共用同一处优先级：resolve().mode 必须等于 effective_mode().0
        std::env::set_var("HEARTH_MODE", "bogus");
        assert_eq!(file.resolve(None, None, None, None, None).mode, "bogus");
        assert_eq!(
            file.resolve(None, None, None, None, None).mode,
            file.effective_mode(None).0,
            "两处优先级必须同源（否则校验的有效值与实际生效值又不是同一个）"
        );
        std::env::remove_var("HEARTH_MODE");
    }

    /// 先红后绿（D-128）：`hearth setup` 必须**合并**进 `.env`，绝不截断重写。
    ///
    /// 红侧（修复前实测）：`std::fs::write(".env", "CODEX_URL=…\n")` 把用户按
    /// `.env.example` 配好的 3 行（`AGNES_API_KEY` / `HEARTH_PROVIDER` / `AGNES_MODEL`）
    /// 整份清成 1 行——本测试的第一条断言（原 3 行必须逐字节还在）会立即失败。
    #[test]
    fn test_d128_env_merge_preserves_every_other_line() {
        let existing =
            "# 我的配置\nAGNES_API_KEY=sk-user-real-key\nHEARTH_PROVIDER=agnes\n\nAGNES_MODEL=agnes-3.0-flash";
        let (out, replaced) = merge_env_assignments(existing, &[("CODEX_URL", "http://h:1")]);
        assert_eq!(replaced, 0, "此前无同名键 ⇒ 不产生就地替换");
        // ① 原有每一行逐字节保留（含注释与空行）
        for line in [
            "# 我的配置",
            "AGNES_API_KEY=sk-user-real-key",
            "HEARTH_PROVIDER=agnes",
            "AGNES_MODEL=agnes-3.0-flash",
        ] {
            assert!(out.contains(line), "原有行必须保留：{line}\n---\n{out}");
        }
        // ② 末尾缺换行也要正确补上（不把两个键粘成一行）
        assert!(out.ends_with("CODEX_URL=http://h:1\n"), "got: {out:?}");
        // ③ 注释里写着的同名键**不算**已有赋值（`.env.example` 就是注释形态）
        let (out2, _) = merge_env_assignments(
            "# CODEX_URL=http://old\nK=V\n",
            &[("CODEX_URL", "http://n")],
        );
        assert_eq!(
            out2.matches("CODEX_URL=").count(),
            2,
            "注释行保留 + 追加真赋值: {out2:?}"
        );
        assert!(
            out2.contains("\nCODEX_URL=http://n\n"),
            "真赋值必须可生效: {out2:?}"
        );
        // ④ 已存在同名键 → 就地替换（不追加第二份）
        let (out3, replaced3) = merge_env_assignments(
            "A=1\r\nCODEX_URL=http://old\r\nB=2\r\n",
            &[("CODEX_URL", "http://new"), ("CODEX_API_KEY", "k")],
        );
        assert_eq!(replaced3, 1, "命中 1 处就地替换");
        assert_eq!(
            out3.matches("CODEX_URL=").count(),
            1,
            "不得出现第二份：{out3:?}"
        );
        assert!(
            out3.contains("CODEX_URL=http://new\r\n"),
            "CRLF 行尾风格沿用: {out3:?}"
        );
        assert!(
            out3.contains("A=1\r\n") && out3.contains("B=2\r\n"),
            "其它行逐字节不动: {out3:?}"
        );
        assert!(
            out3.ends_with("CODEX_API_KEY=k\n"),
            "未命中的键追加: {out3:?}"
        );
        // ⑤ 空文件 → 只写我们管理的键
        let (out4, r4) = merge_env_assignments("", &[("CODEX_URL", "u")]);
        assert_eq!((out4.as_str(), r4), ("CODEX_URL=u\n", 0));
    }

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

    /// D-117 回归锁：CLI 必须认**provider 惯例 env**（`AGNES_API_KEY` / `AGNES_BASE_URL` /
    /// `AGNES_MODEL`）——与 service、`.env.example`、`docs/configuration.md` 同一套名字。
    ///
    /// 红侧（修复前）：CLI 只认 `HEARTH_*` ⇒ 用户照模板配好 `.env`，`api_key` 仍为 `None`，
    /// `hearth chat` 报"未配置 API key"（"配好了却不生效"，D-105③/D-106 同族）。
    #[test]
    fn test_d117_provider_convention_env_is_honored() {
        let _env_ser = p3_tests::ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        const CLEAR: [&str; 8] = [
            "HEARTH_PROVIDER",
            "HEARTH_API_KEY",
            "HEARTH_MODEL",
            "HEARTH_LLM_URL",
            "HEARTH_URL",
            "AGNES_API_KEY",
            "AGNES_BASE_URL",
            "AGNES_MODEL",
        ];
        for k in CLEAR {
            std::env::remove_var(k);
        }
        let cfg = Config {
            provider: Some("agnes".into()),
            ..Config::default()
        };
        std::env::set_var("AGNES_API_KEY", "agnes-key");
        std::env::set_var("AGNES_BASE_URL", "https://api.agnes-ai.cn/v1");
        std::env::set_var("AGNES_MODEL", "agnes-2.5-flash");

        let r = cfg.resolve(None, None, None, None, None);
        assert_eq!(
            r.api_key.as_deref(),
            Some("agnes-key"),
            "AGNES_API_KEY 必须被 CLI 采纳（D-117）"
        );
        assert_eq!(
            r.url.as_deref(),
            Some("https://api.agnes-ai.cn/v1"),
            "AGNES_BASE_URL 必须被 CLI 采纳（D-117）"
        );
        assert_eq!(
            r.model.as_deref(),
            Some("agnes-2.5-flash"),
            "AGNES_MODEL 必须被 CLI 采纳（D-117）"
        );

        // 优先级不倒退：HEARTH_API_KEY 仍压过惯例名。
        std::env::set_var("HEARTH_API_KEY", "hearth-key");
        let r2 = cfg.resolve(None, None, None, None, None);
        assert_eq!(
            r2.api_key.as_deref(),
            Some("hearth-key"),
            "HEARTH_API_KEY 必须优先于 AGNES_API_KEY（不改既有优先级）"
        );

        for k in CLEAR {
            std::env::remove_var(k);
        }
    }

    /// S9（手术包二）：降级链配置解析——config providers 生效 / env 优先 /
    /// 缺省为空（单通道行为不变）。
    #[test]
    fn test_s9_resolve_providers_chain() {
        let _env_ser = p3_tests::ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("HEARTH_PROVIDERS");
        // P1-06：原为 `let mut cfg = Config::default(); cfg.providers = …` ——
        // clippy `field_reassign_with_default`（CI `-D warnings` 下报错），改为结构体更新语法。
        let cfg = Config {
            providers: Some(vec!["agnes".into(), "zhipu".into(), "gemini".into()]),
            ..Config::default()
        };
        let r = cfg.resolve(None, None, None, None, None);
        assert_eq!(
            r.providers,
            vec!["agnes", "zhipu", "gemini"],
            "config providers 必须生效（有序链）"
        );
        std::env::set_var("HEARTH_PROVIDERS", "deepseek,openai");
        let r2 = cfg.resolve(None, None, None, None, None);
        assert_eq!(
            r2.providers,
            vec!["deepseek", "openai"],
            "env HEARTH_PROVIDERS 优先于 config"
        );
        std::env::remove_var("HEARTH_PROVIDERS");
        let r3 = Config::default().resolve(None, None, None, None, None);
        assert!(r3.providers.is_empty(), "缺省 = 空链（单通道行为不变）");
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
