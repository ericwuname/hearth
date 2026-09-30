//! D-6（2026-10-01, traecode）：**入库前密钥扫描门禁**。
//!
//! # 为什么需要（根因，不是"最佳实践"）
//!
//! 2026-09-30 公开仓库明文密钥事故：`6871f7a` 的一次"文档归档"提交，把两天前
//! 已脱敏的三把明文 key **重新引入**仓库并公开约 10 天。复盘结论是——
//! **脱敏是一次性人工动作，没有任何常设门禁**：只要有人再提交一次含 key 的文件，
//! 同样的事就会再发生一次。
//!
//! 本测试把"扫描"固化成**常设门禁**：扫描 **git 已跟踪** 的全部文本文件
//! （= 会被入库的内容），命中高置信密钥模式即**失败**。
//!
//! # 白名单（ACCEPTED）
//!
//! 对**已由顶层裁决接受风险**的既有命中做显式白名单（文件 → 允许的命中数）。
//! 白名单的每一项都必须能在《P0安全事故报告·公开仓库明文密钥泄露 v1.0》
//! （仓库根目录，**有意不 commit**）中找到对应裁决。
//! **任何新增命中**（含"同一文件命中数变多"）都会让本测试变红。
//!
//! # 扫描口径
//!
//! - 只看 `git ls-files` 的结果 —— 运行期产物（如 `.hearth-diag/`，见 D-5）不在内；
//! - 非 UTF-8（二进制）文件**跳过但计数并打印**（不静默）；
//! - 只打印 **文件与命中数**，**绝不打印命中内容**（测试日志也是公开的）。
//!
//! # 已知盲区（**不得**当成"扫过就安全"）
//!
//! 1. 只认**有前缀约定**的密钥（`cpk-`/`sk-`/`ghp_`/`AKIA`/…）+ PEM 私钥块。
//!    自研网关的任意口令、base64 里的凭据、SSH 私钥之外的凭据形态**扫不到**；
//! 2. 二进制/不可读文件跳过（会打印跳过数量）；
//! 3. 只看**已跟踪**内容 —— 未跟踪文件（如 `.env`、本报告正文）不在范围。
//!
//! 这三条是本门禁的**置信边界**：它在"已知形态"上是硬门禁，不是全量 DLP。

use std::process::Command;

/// 高置信密钥前缀。要求前缀后至少 16 个 [A-Za-z0-9_-] 才算"疑似完整 key"
/// —— 这样写文档时用的脱敏形式（如 `cpk-f4UBH3Na…`，仅 9 字符）不会被误报。
///
/// 覆盖范围说明（**不是"所有密钥"，是"高置信、低误报的一批"**）：
/// 本仓实际用过的 `cpk-` / `sk-`，加上公开已知的常见 provider/平台前缀。
/// 其余形态（自研网关的任意口令等）无法用前缀识别 —— 见文件头"已知盲区"。
const KEY_PREFIXES: [&str; 9] = [
    "cpk-",     // 本仓实际使用（agnes/api hub）
    "sk-",      // OpenAI 系
    "sk_live_", // Stripe
    "ghp_",     // GitHub PAT
    "gho_", "xoxb-", // Slack
    "xoxp-", "AKIA", // AWS access key id
    "AIza", // Google API key
];

/// 占位符关键词（大小写不敏感）：命中体里出现即判定为**示例/占位**，不算密钥。
/// 依据：本次实测两个高频误报源——`.env.example` 的 `sk-your-real-deepseek-key`
/// 与契约测试的 `sk-test-placeholder-for-contract-test`。
const PLACEHOLDER_MARKERS: [&str; 6] = [
    "your",
    "test",
    "example",
    "placeholder",
    "dummy",
    "changeme",
];

/// 判定一个"前缀 + 长串"是否**像真实密钥**（而非占位符/散文）。
///
/// 真实 provider key 的body 必然**含数字**（base62/base64/hex 都是），
/// 而英文占位串（`your-real-deepseek-key-if-needed`）通常不含 —— 这条规则
/// 把"文档里写给人看的示例"与"机器生成的口令"分开了。
fn looks_like_secret(body: &str) -> bool {
    if !body.bytes().any(|b| b.is_ascii_digit()) {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    !PLACEHOLDER_MARKERS.iter().any(|m| lower.contains(m))
}

/// 已裁决接受风险的命中（文件 → 允许命中数）。
///
/// 全部来自 2026-09-30 事故已公开密钥的**既有**痕迹，用户裁决：
/// **不轮换、保持 public、不做历史清理**（显式风险接受）。
/// 新增条目必须同步更新事故报告与规划债队列，不允许"为了让测试变绿"随手加。
const ACCEPTED: [(&str, usize); 3] = [
    // 3 把真 key 的**原始落点**（实测：同一把 cpk 出现两次，故计数 4）
    (
        "docs/p0-usability/r9-reports/data/r12-dot-bashrc-anchor.txt",
        4,
    ),
    // 录屏文字稿里带出的同一把 cpk（用户当时粘贴的目标串）
    (
        "docs/p0-usability/r9-reports/data/v20-user-transcript-f852f409.jsonl",
        1,
    ),
    // 验收报告里作为"发现项"引用的同一把 cpk
    (
        "docs/p0-usability/r9-reports/TUI-polish3发射 + 用户真机会话体感定案 + D14复证与闭环 v1.1（评审窗→顶层）.md",
        1,
    ),
];

fn repo_root() -> std::path::PathBuf {
    // crates/codex-cli → ../..
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// 统计 `text` 中形如 `<prefix><16+ 个 [A-Za-z0-9_-]>` 且 [`looks_like_secret`]
/// 成立的出现次数。
fn count_hits(text: &str, prefix: &str) -> usize {
    let bytes = text.as_bytes();
    let mut hits = 0usize;
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(prefix) {
        let at = from + rel;
        let start = at + prefix.len();
        // 词边界：前缀左边不得是字母/数字/`-`/`_`。否则会把普通英文词里的
        // "sk-" 也算进去 —— 实测最大误报源是 `task-order-v16-….md`
        //（"ta**sk-**order" 后接版本号，含数字且够长）。
        let prev_ok = at == 0 || {
            let p = bytes[at - 1];
            !(p.is_ascii_alphanumeric() || p == b'-' || p == b'_')
        };
        let mut j = start;
        while j < bytes.len()
            && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-' || bytes[j] == b'_')
        {
            j += 1;
        }
        if prev_ok && j - start >= 16 && looks_like_secret(&text[start..j]) {
            hits += 1;
        }
        from = start; // 继续向后找（start 必为字符边界：紧随 ASCII 前缀）
    }
    hits
}

#[test]
fn secret_scan_gate() {
    let root = repo_root();
    let out = match Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "-z"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            // 不静默跳过：环境不具备时就明确说出来（而非假装通过）。
            eprintln!("SKIP secret_scan_gate: 无法执行 git（{e}）——本机无法做入库前扫描");
            return;
        }
    };
    if !out.status.success() {
        eprintln!(
            "SKIP secret_scan_gate: `git ls-files` 失败（{}）——不在 git 工作区？",
            out.status
        );
        return;
    }

    let files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    assert!(
        !files.is_empty(),
        "git ls-files 返回空——扫描无意义，不得当作通过"
    );

    let mut offenders: Vec<(String, usize)> = Vec::new();
    let mut skipped_binary = 0usize;

    for rel in &files {
        let path = root.join(rel);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                skipped_binary += 1; // 二进制/不可读：跳过但计数（下面会打印）
                continue;
            }
        };
        let mut hits: usize = KEY_PREFIXES.iter().map(|p| count_hits(&text, p)).sum();
        // PEM 私钥块：一眼可判、零误报。
        if text.contains("PRIVATE KEY-----") {
            hits += 1;
        }
        if hits == 0 {
            continue;
        }
        let allowed = ACCEPTED
            .iter()
            .find(|(f, _)| f == rel)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        if hits > allowed {
            offenders.push((rel.clone(), hits));
        }
    }

    if skipped_binary > 0 {
        eprintln!(
            "note: {skipped_binary} 个非文本文件已跳过（二进制内容不做扫描）——如需覆盖请另立卡"
        );
    }

    assert!(
        offenders.is_empty(),
        "密钥扫描门禁未过：发现 {} 个文件含未登记的疑似明文密钥（只列文件与命中数，不打印内容）：\n{}\n\
         处置：① 删除密钥并改用 env/配置注入；② 若确认属「已裁决接受风险」，\
         需先把该文件加进本测试的 ACCEPTED 白名单并同步事故报告。",
        offenders.len(),
        offenders
            .iter()
            .map(|(f, n)| format!("  - {f}  ×{n}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    eprintln!(
        "secret_scan_gate PASS: 扫描 {} 个已跟踪文件；白名单内 {} 项已裁决接受风险。",
        files.len(),
        ACCEPTED.len()
    );
}
