//! wiring: 接线断言引擎（X3）——防回归防线的物质载体。
//!
//! 背景（docs/project-xray-design.md B.3）：v10.3 修好 constitution 注入，
//! v11.4 无声改坏。没有接线断言，任何"已清零"的债都能在下一版退化。
//!
//! 规格文件为 TOML（工程决策：`toml` 已是 workspace 依赖，零新增依赖；
//! VM 上 github 不可达，新依赖是编译期风险——见 forge-v13-plan §3.1）。
//!
//! 语义：每条 capability 有一条 chain；每环指向一个文件，
//! `all` 中所有子串都命中 且 `any` 中至少一个命中（为空则跳过该组）才算通。
//! 任一环不通 = 该能力断裂；severity=red 的断裂 → exit 1。

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct WiringSpec {
    pub schema: u32,
    #[serde(rename = "capability", default)]
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Deserialize)]
pub struct Capability {
    pub id: String,
    #[serde(default)]
    pub claim: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(rename = "chain", default)]
    pub chain: Vec<ChainLink>,
}

fn default_severity() -> String {
    "red".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ChainLink {
    /// workspace 相对路径。
    pub file: String,
    /// 至少一个子串命中（OR）。空 = 跳过该组。
    #[serde(default)]
    pub any: Vec<String>,
    /// 全部子串命中（AND）。空 = 跳过该组。
    #[serde(default)]
    pub all: Vec<String>,
    /// 这一环在链上的含义（报告用）。
    #[serde(default)]
    pub meaning: String,
}

#[derive(Debug)]
pub struct LinkResult {
    pub file: String,
    pub meaning: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug)]
pub struct CapabilityResult {
    pub id: String,
    pub claim: String,
    pub severity: String,
    pub broken: bool,
    pub links: Vec<LinkResult>,
}

/// 读取并解析规格文件。
pub fn load_spec(path: &Path) -> Result<WiringSpec> {
    let text = fs::read_to_string(path).with_context(|| format!("read spec {}", path.display()))?;
    let spec: WiringSpec =
        toml::from_str(&text).with_context(|| format!("parse spec {}", path.display()))?;
    anyhow::ensure!(
        spec.schema == 1,
        "unsupported wiring schema {}",
        spec.schema
    );
    anyhow::ensure!(
        !spec.capabilities.is_empty(),
        "wiring spec has zero capabilities — refusing a vacuous green gate"
    );
    Ok(spec)
}

/// 对 `root` 下的源码执行全部断言。
pub fn check(root: &Path, spec: &WiringSpec) -> Vec<CapabilityResult> {
    spec.capabilities
        .iter()
        .map(|cap| {
            let links: Vec<LinkResult> = cap
                .chain
                .iter()
                .map(|link| check_link(root, link))
                .collect();
            // 空链 = 无效规约，视为断裂（防止"零断言绿灯"）。
            let broken = links.is_empty() || links.iter().any(|l| !l.ok);
            CapabilityResult {
                id: cap.id.clone(),
                claim: cap.claim.clone(),
                severity: cap.severity.clone(),
                broken,
                links,
            }
        })
        .collect()
}

fn check_link(root: &Path, link: &ChainLink) -> LinkResult {
    let path = root.join(&link.file);
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return LinkResult {
                file: link.file.clone(),
                meaning: link.meaning.clone(),
                ok: false,
                detail: format!("file unreadable: {e}"),
            };
        }
    };

    // P0-4 (audit-fix): 先剥离注释/字符串再做子串匹配——否则把调用整行
    // 注释掉（子串留在注释里）仍能通过 wiring，G1「代码分支必执行」退化为
    // 「文本出现过」。剥离后注释掉的调用不再命中 → 断裂被检出。
    let content = strip_comments_and_strings(&content);

    if link.any.is_empty() && link.all.is_empty() {
        return LinkResult {
            file: link.file.clone(),
            meaning: link.meaning.clone(),
            ok: false,
            detail: "invalid link: no patterns (any/all both empty)".to_string(),
        };
    }

    let missing_all: Vec<&String> = link.all.iter().filter(|p| !content.contains(*p)).collect();
    let any_ok = link.any.is_empty() || link.any.iter().any(|p| content.contains(p));

    let ok = missing_all.is_empty() && any_ok;
    let detail = if ok {
        "ok".to_string()
    } else if !missing_all.is_empty() {
        format!("missing(all): {missing_all:?}")
    } else {
        format!("none of any-patterns found: {:?}", link.any)
    };

    LinkResult {
        file: link.file.clone(),
        meaning: link.meaning.clone(),
        ok,
        detail,
    }
}

/// P0-4 (audit-fix): 剥离 Rust 行注释/块注释，保留字符串字面量。
/// 背景：wiring 断言分两类——(a) 调用语句（`self.record_tool_exchange();`），
/// 注释掉后子串留在注释里仍命中 = 绕过；(b) 字符串/文本断言
/// （`"write_file"` 工具名、`"constitution.md"` 文件名、trait 名），这些**必须**保留。
/// 因此只剥注释：既堵住 (a) 的注释绕过，又不误伤 (b)。
fn strip_comments_and_strings(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let mut in_line = false;
    let mut in_block = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_line {
            if c == b'\n' {
                in_line = false;
                out.push('\n');
            }
            i += 1;
            continue;
        }
        if in_block {
            if c == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                in_block = false;
                i += 2;
                continue;
            }
            if c == b'\n' {
                out.push('\n');
            }
            i += 1;
            continue;
        }
        // 裸代码区：只识别注释入口，其余（含字符串）原样保留
        if c == b'/' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'/' => {
                    in_line = true;
                    i += 2;
                    continue;
                }
                b'*' => {
                    in_block = true;
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// 是否存在 severity=red 的断裂（决定 exit code）。
pub fn has_red_break(results: &[CapabilityResult]) -> bool {
    results.iter().any(|r| r.broken && r.severity == "red")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn strip_removes_comments_and_strings() {
        // 纯注释里的调用子串必须被剥离（P0-4 核心：注释掉调用 ≠ 存在）
        let only_line_comment = "// self.record_tool_exchange();\nlet x = 1;";
        assert!(
            !strip_comments_and_strings(only_line_comment).contains("record_tool_exchange"),
            "行注释中的调用不应命中"
        );
        let only_block_comment = "/* self.record_tool_exchange(); */";
        assert!(
            !strip_comments_and_strings(only_block_comment).contains("record_tool_exchange"),
            "块注释中的调用不应命中"
        );
        // 字符串字面量必须保留（wiring 断言含工具名/文件名等字符串匹配）
        let only_string = "let m = MUTATING_TOOLS: &[\"write_file\", \"bash\"];";
        assert!(
            strip_comments_and_strings(only_string).contains("write_file"),
            "字符串字面量中的断言目标应保留"
        );
        // 真实代码里的调用必须保留
        let real = "fn f() { self.record_tool_exchange(); }";
        assert!(
            strip_comments_and_strings(real).contains("record_tool_exchange"),
            "真实调用应保留"
        );
        // 混合：注释 + 真实调用 → 真实调用命中
        let mixed = "// self.record_tool_exchange();\nself.record_tool_exchange();";
        assert!(
            strip_comments_and_strings(mixed).contains("record_tool_exchange"),
            "混合场景应命中真实调用"
        );
    }

    fn fixture(spec_text: &str, file_content: &str) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/x.rs"), file_content).unwrap();
        let spec_path = root.join("wiring.toml");
        fs::write(&spec_path, spec_text).unwrap();
        (tmp, spec_path)
    }

    const SPEC: &str = r#"
schema = 1
[[capability]]
id = "demo"
claim = "hook is wired"
severity = "red"
[[capability.chain]]
file = "src/x.rs"
any = ["record_tool_exchange("]
meaning = "call site"
"#;

    #[test]
    fn test_wiring_pass() {
        let (tmp, spec_path) = fixture(SPEC, "fn f() { self.record_tool_exchange(); }\n");
        let spec = load_spec(&spec_path).unwrap();
        let results = check(tmp.path(), &spec);
        assert_eq!(results.len(), 1);
        assert!(!results[0].broken);
        assert!(!has_red_break(&results));
    }

    /// 自证测试（forge-v13 验收 🔴）：故意断链必须变红。
    /// 模拟"有人把 record_tool_exchange 调用删了"——断言引擎必须报断裂。
    #[test]
    fn test_self_proof_break_turns_red() {
        let (tmp, spec_path) = fixture(SPEC, "fn f() { /* call removed by refactor */ }\n");
        let spec = load_spec(&spec_path).unwrap();
        let results = check(tmp.path(), &spec);
        assert!(results[0].broken, "broken chain MUST be detected");
        assert!(has_red_break(&results), "red severity MUST fail the gate");
    }

    #[test]
    fn test_missing_file_is_break() {
        let tmp = tempfile::tempdir().unwrap();
        let spec_path = tmp.path().join("wiring.toml");
        fs::write(&spec_path, SPEC).unwrap();
        // src/x.rs 不存在
        let spec = load_spec(&spec_path).unwrap();
        let results = check(tmp.path(), &spec);
        assert!(results[0].broken);
        assert!(has_red_break(&results));
    }

    #[test]
    fn test_all_semantics_requires_every_pattern() {
        let spec_text = r#"
schema = 1
[[capability]]
id = "all-demo"
severity = "red"
[[capability.chain]]
file = "src/x.rs"
all = ["alpha", "beta"]
"#;
        let (tmp, spec_path) = fixture(spec_text, "alpha only\n");
        let spec = load_spec(&spec_path).unwrap();
        let results = check(tmp.path(), &spec);
        assert!(results[0].broken, "missing 'beta' must break the chain");
    }

    #[test]
    fn test_yellow_break_does_not_fail_gate() {
        let spec_text = r#"
schema = 1
[[capability]]
id = "soft"
severity = "yellow"
[[capability.chain]]
file = "src/x.rs"
any = ["nonexistent-pattern"]
"#;
        let (tmp, spec_path) = fixture(spec_text, "nothing here\n");
        let spec = load_spec(&spec_path).unwrap();
        let results = check(tmp.path(), &spec);
        assert!(results[0].broken);
        assert!(!has_red_break(&results), "yellow break must not exit 1");
    }

    #[test]
    fn test_empty_spec_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let spec_path = tmp.path().join("wiring.toml");
        fs::write(&spec_path, "schema = 1\n").unwrap();
        assert!(
            load_spec(&spec_path).is_err(),
            "vacuous green gate rejected"
        );
    }

    #[test]
    fn test_empty_chain_is_break() {
        let spec_text = r#"
schema = 1
[[capability]]
id = "no-chain"
severity = "red"
"#;
        let tmp = tempfile::tempdir().unwrap();
        let spec_path = tmp.path().join("wiring.toml");
        fs::write(&spec_path, spec_text).unwrap();
        let spec = load_spec(&spec_path).unwrap();
        let results = check(tmp.path(), &spec);
        assert!(results[0].broken, "capability with empty chain must break");
    }
}
