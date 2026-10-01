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
    //
    // P0-5 (audit-fix): 再抹掉 `#[cfg(test)]` 区域——剥离器**保留字符串字面量**，
    // 故测试代码里的字符串（如工具名数组）能"顶绿"生产锚点：
    // `readonly-view-strips`（red）锚 `dispatcher.rs` 的 `MUTATING_TOOLS`，
    // 生产数组删掉 `"edit"` 后，测试行 `["…","write_file","edit","bash"]` 仍命中 → 假绿。
    // 先 strip（注释里的 `#[cfg(test)]` 已随之消失）再 blank，双保险。
    let content = blank_test_modules(&strip_comments_keep_strings(&content));

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
/// 剥离过程中的词法状态。**必须跟踪字符串**——否则字符串里的 `/*` 会被误判为
/// 块注释开头，吞掉其后整份文件（实测 `bash.rs` 被吞 79%：43132→9081 字节），
/// 导致该文件上的一切断言**假报缺失**。
#[derive(PartialEq, Clone, Copy)]
enum LexState {
    Code,
    LineComment,
    BlockComment,
    /// 普通字符串 `"..."`（含其 `\` 转义）
    Str,
    /// 字符字面量 `'x'` / `'\n'`（**不含**生命周期 `'a`）
    Char,
    /// 原始字符串 `r"..."` / `r#"..."#`，参数为 `#` 的个数
    RawStr(usize),
}

/// `'` 处是否构成字符字面量（而非生命周期 `'a`）。
///
/// 启发式：向后最多 5 字节内找到闭合 `'`，且期间无换行 → 是字符字面量。
/// `'a` / `&'static str` 这类找不到闭合引号 → 判为生命周期，按普通代码处理。
fn is_char_literal(b: &[u8], i: usize) -> bool {
    let n = b.len();
    let mut j = i + 1;
    if j < n && b[j] == b'\\' {
        j += 2; // 转义：'\n' '\\' '\''
    } else if j < n {
        let lead = b[j];
        let len = if lead < 0x80 {
            1
        } else if lead >> 5 == 0b110 {
            2
        } else if lead >> 4 == 0b1110 {
            3
        } else {
            4
        };
        j += len; // 跳过整个 UTF-8 字符（支持 '中'）
    }
    j < n && b[j] == b'\''
}

/// 剥离行注释与块注释，**保留字符串字面量与其余代码**。
///
/// D-69（2026-10-01）订正：本函数原名 `strip_comments_and_strings`——**名与行为不符**
/// （"strip ... strings" 读起来像"把字符串也剥掉"，实际是**保留**）。这个误导正是
/// D-59「假绿」的认知来源：读者以为字符串已被剥掉、不会再参与锚点匹配，于是没人去想
/// "测试里的字符串能顶绿生产锚点"。**改名为 `strip_comments_keep_strings`**，名实一致。
/// （真正需要"连字符串一起屏蔽"的场景由 [`blank_test_modules`] 负责。）
///
/// 历史修复（均为 traecode，2026-09-30）：
/// 1. **UTF-8 缺陷**：原实现 `out.push(c as char)` 把 u8 当码点，多字节字符（中文）被拆成
///    乱码 → 含非 ASCII 的锚点永远匹配不上（门禁假阴性）。现改为原始字节收集、结尾整体还原。
/// 2. **字符串状态缺陷**：原实现只看 `/`+`*` 两个字节、不区分是否位于字符串内 → 字符串里的
///    `/*` 会开启"块注释"并吞掉其后一切。现引入 [`LexState`] 跟踪普通/原始字符串与字符字面量。
///
/// 语义保持不变：只剥注释，字符串与代码原样保留。
fn strip_comments_keep_strings(src: &str) -> String {
    let b = src.as_bytes();
    let n = b.len();
    let mut out: Vec<u8> = Vec::with_capacity(n);
    let mut i = 0usize;
    let mut st = LexState::Code;

    while i < n {
        let c = b[i];
        match st {
            LexState::LineComment => {
                if c == b'\n' {
                    st = LexState::Code;
                    out.push(b'\n');
                }
                i += 1;
            }
            LexState::BlockComment => {
                if c == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    st = LexState::Code;
                    i += 2;
                } else {
                    if c == b'\n' {
                        out.push(b'\n');
                    }
                    i += 1;
                }
            }
            LexState::Str | LexState::Char => {
                let closing = if st == LexState::Str { b'"' } else { b'\'' };
                out.push(c);
                if c == b'\\' && i + 1 < n {
                    out.push(b[i + 1]); // 转义对整体保留
                    i += 2;
                } else {
                    if c == closing {
                        st = LexState::Code;
                    }
                    i += 1;
                }
            }
            LexState::RawStr(hashes) => {
                out.push(c);
                if c == b'"' {
                    let mut k = 1;
                    while k <= hashes && i + k < n && b[i + k] == b'#' {
                        out.push(b'#');
                        k += 1;
                    }
                    if k == hashes + 1 {
                        out.push(b'"');
                        i += k + 1;
                        st = LexState::Code;
                        continue;
                    }
                    i += k;
                    continue;
                }
                i += 1;
            }
            LexState::Code => {
                // 注释入口
                if c == b'/' && i + 1 < n {
                    match b[i + 1] {
                        b'/' => {
                            st = LexState::LineComment;
                            i += 2;
                            continue;
                        }
                        b'*' => {
                            st = LexState::BlockComment;
                            i += 2;
                            continue;
                        }
                        _ => {}
                    }
                }
                // 原始字符串：r"..." / r#"..."# / r##"..."##
                if c == b'r' {
                    let mut k = i + 1;
                    while k < n && b[k] == b'#' {
                        k += 1;
                    }
                    if k < n && b[k] == b'"' {
                        st = LexState::RawStr(k - (i + 1));
                        i = k + 1;
                        continue;
                    }
                }
                // 普通字符串
                if c == b'"' {
                    st = LexState::Str;
                    out.push(c);
                    i += 1;
                    continue;
                }
                // 字符字面量（排除生命周期）
                if c == b'\'' && is_char_literal(b, i) {
                    st = LexState::Char;
                    out.push(c);
                    i += 1;
                    continue;
                }
                out.push(c);
                i += 1;
            }
        }
    }
    // 只删除 ASCII 注释字节的子序列仍是合法 UTF-8，故此处无损。
    String::from_utf8_lossy(&out).into_owned()
}

/// 给定 `#[cfg(test)]` 之后的字节下标，返回该属性所修饰**条目**的结束下标
/// （配对的 `}` 或该语句的 `;` 之后一位）。找不到（文件异常）→ 返回 `n`
/// （保守：抹到文件末尾——宁可让锚点变红，也不要静默漏抹）。
///
/// 判定"花括号形态 vs 无花括号形态"：扫描中先遇到 `{` → 花括号条目（配平到 `}`）；
/// 先遇到顶层 `;` → 无花括号条目（`use ...;` / `mod foo;`）。
/// 复用 [`LexState`] 词法状态：字符串/原始字符串/字符字面量/注释里的一切
/// （包括 `{` `}` `;`）都**不**参与结构判定。
fn cfg_test_item_end(b: &[u8], n: usize, from: usize) -> usize {
    let mut i = from;
    let mut st = LexState::Code;
    let mut braces = 0usize; // {} 深度
    let mut groups = 0usize; // () / [] 深度（属性、类型/表达式里的分号不误判）
    let mut seen_brace = false;
    while i < n {
        let c = b[i];
        match st {
            LexState::LineComment => {
                if c == b'\n' {
                    st = LexState::Code;
                }
                i += 1;
            }
            LexState::BlockComment => {
                if c == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    st = LexState::Code;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            LexState::Str | LexState::Char => {
                let closing = if st == LexState::Str { b'"' } else { b'\'' };
                if c == b'\\' && i + 1 < n {
                    i += 2;
                } else {
                    if c == closing {
                        st = LexState::Code;
                    }
                    i += 1;
                }
            }
            LexState::RawStr(hashes) => {
                if c == b'"' {
                    let mut k = 1;
                    while k <= hashes && i + k < n && b[i + k] == b'#' {
                        k += 1;
                    }
                    if k == hashes + 1 {
                        st = LexState::Code;
                        i += k + 1;
                        continue;
                    }
                }
                i += 1;
            }
            LexState::Code => {
                if c == b'/' && i + 1 < n {
                    match b[i + 1] {
                        b'/' => {
                            st = LexState::LineComment;
                            i += 2;
                            continue;
                        }
                        b'*' => {
                            st = LexState::BlockComment;
                            i += 2;
                            continue;
                        }
                        _ => {}
                    }
                }
                if c == b'r' {
                    let mut k = i + 1;
                    while k < n && b[k] == b'#' {
                        k += 1;
                    }
                    if k < n && b[k] == b'"' {
                        st = LexState::RawStr(k - (i + 1));
                        i = k + 1;
                        continue;
                    }
                }
                if c == b'"' {
                    st = LexState::Str;
                    i += 1;
                    continue;
                }
                if c == b'\'' && is_char_literal(b, i) {
                    st = LexState::Char;
                    i += 1;
                    continue;
                }
                match c {
                    b'{' => {
                        braces += 1;
                        seen_brace = true;
                        i += 1;
                    }
                    b'}' => {
                        braces = braces.saturating_sub(1);
                        i += 1;
                        if seen_brace && braces == 0 {
                            return i;
                        }
                    }
                    b'(' | b'[' => {
                        groups += 1;
                        i += 1;
                    }
                    b')' | b']' => {
                        groups = groups.saturating_sub(1);
                        i += 1;
                    }
                    b';' if !seen_brace && groups == 0 => return i + 1,
                    _ => i += 1,
                }
            }
        }
    }
    n
}

/// 把源码里所有 **`#[cfg(test)]` 覆盖的区域**整体抹成空白（保留换行，行号对齐）。
///
/// 目的（P0-5 audit-fix）：让 wiring 锚点**对测试代码免疫**——测试里出现的字符串
/// （如工具名数组）不得"顶绿"生产锚点。反例：`readonly-view-strips`（red）锚定
/// `dispatcher.rs` 的生产数组 `MUTATING_TOOLS`，但同文件测试里的
/// `["read","grep","glob","write_file","edit","bash"]` 含同名子串，
/// 生产数组即便删掉 `"edit"` 仍假绿。抹掉测试区后，此漏洞被堵死。
///
/// 形态：
/// - `#[cfg(test)] mod tests { ... }`（任意嵌套/字符串/注释）→ 抹到配对的 `}`；
/// - `#[cfg(test)] use ...;` / `#[cfg(test)] mod foo;` 等无花括号形态 → 抹到该语句的 `;`；
/// - 找不到匹配右括号（文件异常）→ 保守抹到文件末尾。
///
/// 复用同文件 [`LexState`] 词法状态机：字符串/注释里出现的 `#[cfg(test)]` **不**触发抹除；
/// 被抹区域内的 `{` `}` 若位于字符串/注释里也**不**参与括号配对。
fn blank_test_modules(src: &str) -> String {
    const MARKER: &[u8] = b"#[cfg(test)]";
    let b = src.as_bytes();
    let n = b.len();
    let mut out: Vec<u8> = Vec::with_capacity(n);
    let mut i = 0usize;
    let mut st = LexState::Code;

    while i < n {
        let c = b[i];
        match st {
            LexState::LineComment => {
                out.push(c);
                if c == b'\n' {
                    st = LexState::Code;
                }
                i += 1;
            }
            LexState::BlockComment => {
                if c == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    out.push(b'*');
                    out.push(b'/');
                    st = LexState::Code;
                    i += 2;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            LexState::Str | LexState::Char => {
                let closing = if st == LexState::Str { b'"' } else { b'\'' };
                out.push(c);
                if c == b'\\' && i + 1 < n {
                    out.push(b[i + 1]);
                    i += 2;
                } else {
                    if c == closing {
                        st = LexState::Code;
                    }
                    i += 1;
                }
            }
            LexState::RawStr(hashes) => {
                out.push(c);
                if c == b'"' {
                    let mut k = 1;
                    while k <= hashes && i + k < n && b[i + k] == b'#' {
                        out.push(b'#');
                        k += 1;
                    }
                    if k == hashes + 1 {
                        out.push(b'"');
                        i += k + 1;
                        st = LexState::Code;
                        continue;
                    }
                    i += k;
                    continue;
                }
                i += 1;
            }
            LexState::Code => {
                // 注释入口：必须先于 marker 判定，保证注释里的 `#[cfg(test)]` 不触发抹除。
                if c == b'/' && i + 1 < n {
                    match b[i + 1] {
                        b'/' => {
                            st = LexState::LineComment;
                            i += 2;
                            continue;
                        }
                        b'*' => {
                            st = LexState::BlockComment;
                            i += 2;
                            continue;
                        }
                        _ => {}
                    }
                }
                // marker 只在 Code 态触发（字符串/字符/原始字符串态不走此分支）。
                if c == b'#' && b[i..].starts_with(MARKER) {
                    let end = cfg_test_item_end(b, n, i + MARKER.len());
                    // 抹成空白但保留换行——报错行号仍与实际源码对齐。
                    for &x in &b[i..end] {
                        out.push(if x == b'\n' { b'\n' } else { b' ' });
                    }
                    i = end;
                    continue;
                }
                // 原始字符串
                if c == b'r' {
                    let mut k = i + 1;
                    while k < n && b[k] == b'#' {
                        k += 1;
                    }
                    if k < n && b[k] == b'"' {
                        st = LexState::RawStr(k - (i + 1));
                        i = k + 1;
                        continue;
                    }
                }
                if c == b'"' {
                    st = LexState::Str;
                    out.push(c);
                    i += 1;
                    continue;
                }
                if c == b'\'' && is_char_literal(b, i) {
                    st = LexState::Char;
                    out.push(c);
                    i += 1;
                    continue;
                }
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
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
            !strip_comments_keep_strings(only_line_comment).contains("record_tool_exchange"),
            "行注释中的调用不应命中"
        );
        let only_block_comment = "/* self.record_tool_exchange(); */";
        assert!(
            !strip_comments_keep_strings(only_block_comment).contains("record_tool_exchange"),
            "块注释中的调用不应命中"
        );
        // 字符串字面量必须保留（wiring 断言含工具名/文件名等字符串匹配）
        let only_string = "let m = MUTATING_TOOLS: &[\"write_file\", \"bash\"];";
        assert!(
            strip_comments_keep_strings(only_string).contains("write_file"),
            "字符串字面量中的断言目标应保留"
        );
        // 真实代码里的调用必须保留
        let real = "fn f() { self.record_tool_exchange(); }";
        assert!(
            strip_comments_keep_strings(real).contains("record_tool_exchange"),
            "真实调用应保留"
        );
        // 混合：注释 + 真实调用 → 真实调用命中
        let mixed = "// self.record_tool_exchange();\nself.record_tool_exchange();";
        assert!(
            strip_comments_keep_strings(mixed).contains("record_tool_exchange"),
            "混合场景应命中真实调用"
        );
    }

    /// 2026-09-30 修复回归（先红后绿）：非 ASCII（CJK）字符必须原样保留。
    ///
    /// 修复前实现 `out.push(c as char)`：`c` 是 UTF-8 的**单字节**，按码点转 char
    /// 会把多字节字符拆成乱码（`编` = E7 BC 96 → 3 个 Latin-1 字符）。后果：
    /// **ASCII 锚点照常通过，含中文的锚点永远匹配不上** —— 门禁假阴性。
    /// 本测试在修复前必红（`contains("编译错误结构化提取（read_lints）")` 为 false）。
    #[test]
    fn strip_preserves_non_ascii_utf8() {
        // ① 字符串字面量中的中文必须原样保留（可被 wiring 锚点匹配）
        let cjk_in_string =
            r#"let s = String::from("[lints] 编译错误结构化提取（read_lints）:\n");"#;
        assert!(
            strip_comments_keep_strings(cjk_in_string).contains("编译错误结构化提取（read_lints）"),
            "字符串字面量中的中文必须原样保留——修复前会被拆成乱码导致锚点静默失效"
        );
        // ② 注释中的中文仍须被剥离（修复不得误放行）
        let cjk_in_comment = "// 编译错误结构化提取（read_lints）\nlet x = 1;";
        assert!(
            !strip_comments_keep_strings(cjk_in_comment).contains("编译错误结构化提取"),
            "注释中的中文仍须剥离"
        );
        // ③ 中英混排：代码/字符串区保留，注释区剥离
        let mixed = "fn f() { /* 中文注释 */ let s = \"中文断言\"; }";
        assert!(strip_comments_keep_strings(mixed).contains("中文断言"));
        assert!(!strip_comments_keep_strings(mixed).contains("中文注释"));
    }

    /// 2026-09-30 修复回归（**先红后绿**）：字符串里的 `/*` 不得被误判为块注释开头。
    ///
    /// 修复前该函数不跟踪字符串状态 → 字符串中的 `/*` 开启"块注释"并吞掉其后一切。
    /// 实测后果：`crates/tools-builtin/src/bash.rs` 剥离后仅剩 9081 字节 / 原文 43132
    /// （**79% 被吞**）、收尾仍停在块注释态 → 该文件上一切锚点**假报缺失**。
    #[test]
    fn strip_does_not_open_block_comment_inside_string() {
        let src = "let re = \"a/* b\";\nlet anchor = \"SHOULD_SURVIVE\";";
        let out = strip_comments_keep_strings(src);
        assert!(
            out.contains("SHOULD_SURVIVE"),
            "字符串里的 /* 不得吞掉其后代码（修复前必红）：{out}"
        );
        assert!(out.contains("a/* b"), "字符串内容须原样保留：{out}");
    }

    /// 原始字符串 / 字符字面量 / 生命周期 三种易误判场景的守护。
    #[test]
    fn strip_handles_raw_strings_chars_and_lifetimes() {
        // ① 原始字符串里的 `/*` 不得开启块注释
        let raw = "let p = r#\"x/*y\"#;\nlet k = \"AFTER_RAW\";";
        assert!(
            strip_comments_keep_strings(raw).contains("AFTER_RAW"),
            "原始字符串里的 /* 不得吞后续"
        );
        // ② 行注释仍须被剥离（修复不得误放行）
        let commented = "let a = 1;\n// GONE_LINE\nlet b = 2;";
        let out = strip_comments_keep_strings(commented);
        assert!(!out.contains("GONE_LINE"), "行注释仍须剥离：{out}");
        assert!(out.contains("let b = 2;"));
        // ③ 生命周期 `'a` 不是字符字面量：不得干扰其后的注释剥离
        let life = "fn f<'a>(x: &'a str) {}\n// LIFETIME_LINE\nlet y = 1;";
        let out2 = strip_comments_keep_strings(life);
        assert!(
            !out2.contains("LIFETIME_LINE"),
            "生命周期不得干扰注释剥离：{out2}"
        );
        assert!(out2.contains("let y = 1;"));
        // ④ 块注释仍须被剥离
        let blk = "/* BLOCK_GONE */\nlet z = 3;";
        let out3 = strip_comments_keep_strings(blk);
        assert!(!out3.contains("BLOCK_GONE"), "块注释仍须剥离：{out3}");
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

    /// P0-5 (audit-fix) 守卫①：生产锚点不因测试代码被抹而丢失。
    ///
    /// 生产数组完整、测试里有同名子串 → 抹除测试区后，生产锚点**仍命中**。
    #[test]
    fn blank_test_modules_keeps_production_anchor_hit() {
        let src = r#"
pub const MUTATING_TOOLS: &[&str] = &["write_file", "edit", "apply_patch", "bash"];

pub fn read_only_view() {}

#[cfg(test)]
mod tests {
    #[test]
    fn strips() {
        let names = ["write_file", "edit", "apply_patch", "bash"];
        assert_eq!(names.len(), 4);
    }
}
"#;
        let out = blank_test_modules(src);
        for p in [
            "MUTATING_TOOLS",
            "\"write_file\"",
            "\"edit\"",
            "\"apply_patch\"",
            "\"bash\"",
            "fn read_only_view",
        ] {
            assert!(out.contains(p), "生产锚点应仍命中 `{p}`：\n{out}");
        }
        // 测试区被抹：测试专属标识不得残留。
        assert!(!out.contains("assert_eq!"), "测试代码应被抹除：\n{out}");
        // 行数不变（报错行号对齐）。
        assert_eq!(
            out.lines().count(),
            src.lines().count(),
            "抹除必须保留换行数"
        );
    }

    /// P0-5 (audit-fix) 守卫②（修复目标）：测试里的字符串**不得**顶绿生产锚点。
    ///
    /// 生产数组删掉 `"edit"`、仅测试里还有 → 抹除后锚点**必须不命中**。
    /// 修复前（只 strip、不 blank）本测试的 `\x22edit\x22` 会由测试行命中 → 假绿。
    #[test]
    fn blank_test_modules_immunizes_anchor_against_test_strings() {
        let src = r#"pub const MUTATING_TOOLS: &[&str] = &["write_file", "apply_patch", "bash"];

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let names = ["write_file", "edit", "apply_patch", "bash"];
        assert!(names.contains(&"edit"));
    }
}
"#;
        let out = blank_test_modules(src);
        assert!(out.contains("MUTATING_TOOLS"), "生产数组仍在");
        assert!(
            !out.contains("\"edit\""),
            "仅测试里存在的 \"edit\" 不得顶绿生产锚点：\n{out}"
        );
        assert!(out.contains("\"apply_patch\""), "生产元素仍应保留");
        // 对照：不抹除时（只 strip）确实会命中——证明本测试揭示的正是假绿路径。
        assert!(
            strip_comments_keep_strings(src).contains("\"edit\""),
            "对照：未抹测试区时 \"edit\" 会被测试行命中（假绿根源）"
        );
    }

    /// P0-5 (audit-fix) 守卫③：字符串/注释里的 `#[cfg(test)]` 不触发抹除。
    #[test]
    fn blank_test_modules_ignores_marker_in_strings_and_comments() {
        let in_string = "let s = \"#[cfg(test)]\";\nlet keep = \"PRODUCTION_ANCHOR\";\n";
        let out = blank_test_modules(in_string);
        assert!(
            out.contains("PRODUCTION_ANCHOR"),
            "字符串里的 marker 不得触发抹除：\n{out}"
        );

        let in_line_comment = "// #[cfg(test)]\nlet keep2 = \"PRODUCTION_ANCHOR_2\";\n";
        let out2 = blank_test_modules(in_line_comment);
        assert!(
            out2.contains("PRODUCTION_ANCHOR_2"),
            "行注释里的 marker 不得触发抹除：\n{out2}"
        );

        let in_block_comment = "/* #[cfg(test)] { } ; */\nlet keep3 = \"PRODUCTION_ANCHOR_3\";\n";
        let out3 = blank_test_modules(in_block_comment);
        assert!(
            out3.contains("PRODUCTION_ANCHOR_3"),
            "块注释里的 marker 不得触发抹除：\n{out3}"
        );

        // 被抹区域内的 {} 若在字符串里，不得参与配对（否则会提前收尾、漏抹）。
        let braces_in_string =
            "let s = \"}{\";\n#[cfg(test)]\nmod tests {\n    fn t() {}\n}\nlet after = \"TAIL\";\n";
        let out4 = blank_test_modules(braces_in_string);
        assert!(
            !out4.contains("fn t()"),
            "字符串里的 }} 不得让配对提前收尾：\n{out4}"
        );
        assert!(out4.contains("TAIL"), "后续生产代码须保留：\n{out4}");
    }
}
