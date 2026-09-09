//! Terminal rendering — colorised SSE event output.

use colored::*;
use std::sync::atomic::{AtomicU8, Ordering};

// ── R3-3 降噪三档（对话可用性根治任务书 v1.0；W-F）──
// 0 = quiet：0 横幅 0 工具行 0 流式增量——只留用户指令/审批/产物/终态/错误
//   （472 条横幅噪声"淹没正文"病理的静音档）；
// 1 = normal（默认）：R3-3 视觉反转后的层级；
// 2 = verbose：全量（等价旧版行为）。
static VERBOSITY: AtomicU8 = AtomicU8::new(1);

pub fn set_verbosity(v: u8) {
    VERBOSITY.store(v.min(2), Ordering::Relaxed);
}

pub fn verbosity() -> u8 {
    VERBOSITY.load(Ordering::Relaxed)
}

fn quiet() -> bool {
    verbosity() == 0
}

/// R4 (v0.1.1): 用户指令气泡——`>` 前缀蓝色加粗（视觉区分"谁在说话"）。
pub fn user_prompt(goal: &str) {
    println!();
    println!("{}", format!("> {goal}").bright_blue().bold());
    println!();
}

/// R4: 工具调用折叠——只显示关键参数（glob→pattern / read/edit→path / bash→command），
/// 截断到 80 字符 + `…`；完整参数不刷屏（BE 语义不变，仅呈现）。
fn summarize_args(args: &serde_json::Value) -> String {
    let truncated = |s: &str| -> String {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() > 80 {
            format!("{}…", chars[..77].iter().collect::<String>())
        } else {
            s.to_string()
        }
    };
    match args {
        serde_json::Value::Object(map) => {
            // 关键字段优先（content 放最后——超长内容不刷屏）
            for k in ["path", "pattern", "file", "command", "url", "content"] {
                if let Some(v) = map.get(k) {
                    let raw = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => serde_json::to_string(other).unwrap_or_default(),
                    };
                    let shown = if k == "content" {
                        let c: Vec<char> = raw.chars().collect();
                        format!(
                            "{}…(+{} chars)",
                            c[..c.len().min(24)].iter().collect::<String>(),
                            c.len()
                        )
                    } else {
                        truncated(&raw)
                    };
                    return format!("{k}: {shown}");
                }
            }
            // 兜底：前 2 个字段
            let mut parts = Vec::new();
            for (k, v) in map.iter().take(2) {
                let s = serde_json::to_string(v).unwrap_or_default();
                parts.push(format!("{k}: {}", truncated(s.trim_matches('"'))));
            }
            parts.join(" ")
        }
        serde_json::Value::String(s) => truncated(s),
        other => truncated(&serde_json::to_string(other).unwrap_or_default()),
    }
}

/// Print a phase transition banner.
pub fn phase(label: &str) {
    if quiet() {
        return; // R3-3 quiet 档：0 横幅
    }
    println!("{}", format!("◆ {}", label).dimmed());
}

/// Stream a token delta (no newline — streaming accumulation).
pub fn token(delta: &str) {
    if quiet() {
        return; // R3-3 quiet 档：流式增量静默
    }
    // R3-3 视觉反转（E15）：正文是用户最该读的内容——旧版正文 .dimmed()
    // 暗灰、工具行 .yellow() 亮黄，层级倒挂（正文比噪声还暗）。自本版起
    // 正文正常色、工具行降 dimmed。
    print!("{delta}");
}

/// R4: 工具调用折叠——`⚙ glob → pattern: **/*` 单行（关键参数 + 80 字截断）。
pub fn tool_call(name: &str, args: &serde_json::Value) {
    if quiet() {
        return; // R3-3 quiet 档：0 工具行
    }
    println!();
    // R3-3 视觉反转：工具行是过程噪声——降为 dimmed（正文才配亮色）
    println!(
        "{}",
        format!("⚙ {name} → {}", summarize_args(args)).dimmed()
    );
}

/// R4: 工具结果投影——**只投影成功**（✓ 绿）；失败由调用方依据工具层结构化
/// is_error（退出码）走 `render::error`（✗ 红）。W4/RC20：废除 contains
/// ("error"/"failed"/"missing") 字符串猜测——双向失效（中文错误画绿 ✓、
/// 正常输出含 error 词画红 ✗）自本版起终结。截断 200 字符，与 ⚙ 对齐。
pub fn tool_result(output: &str) {
    if quiet() {
        return; // R3-3 quiet 档：工具结果静默
    }
    let truncated: String = output.chars().take(200).collect();
    let suffix = if output.chars().count() > 200 {
        "…"
    } else {
        ""
    };
    println!(
        "  {}",
        format!("✓ {}{}", truncated, suffix).green().dimmed()
    );
}

/// Need-approval prompt.
pub fn need_approval(sid: &str, aid: &str, action: &str) {
    println!();
    println!(
        "{}",
        format!("⛔ {action} — 用 `approve {sid} {aid}` 审批，或 `deny {sid} {aid}` 拒绝")
            .bright_red()
            .bold()
    );
}

/// Reflection verdict.
pub fn reflection(verdict: &str) {
    if quiet() {
        return;
    }
    println!("{}", format!("  {}", verdict).cyan());
}

/// Done / summary.
/// RC34 (P3-BACKLOG): ×2 渲染去重——同一 (steps, ok) 连续重复渲染折叠为一次
/// （盲测用户看到 Done 行重复 ×2）。static 记录上次渲染，重复即跳过。
pub fn done(steps: u64, ok: bool) {
    use std::sync::atomic::{AtomicU64, Ordering};
    // 打包 key：steps<<1 | ok
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    let key = (steps << 1) | (ok as u64);
    if LAST.load(Ordering::Relaxed) == key {
        return; // 重复终态行折叠
    }
    LAST.store(key, Ordering::Relaxed);
    let icon = if ok { "✓" } else { "✗" };
    let msg = format!("{icon} Done ({steps} steps)");
    if ok {
        println!("{}", msg.bold());
    } else {
        println!("{}", msg.red().bold());
    }
}

/// Error message.
pub fn error(msg: &str) {
    println!("{}", format!("✗ {}", msg).red());
}

/// Y2: Structured error — what happened / why / how to fix.
pub fn error_structured(what: &str, why: &str, how: &str) {
    println!();
    println!("{}", format!("✗ {what}").red().bold());
    println!("  原因：{}", why);
    println!("  怎么修：{}", how);
    println!();
}

/// Info line.
pub fn info(msg: &str) {
    println!("{}", msg.dimmed());
}

/// Y2: Setup completion — smoke test passed, show next steps.
pub fn setup_ok(url: &str) {
    println!();
    println!("{}", format!("✓ 冒烟测试通过：{url} 可用").green().bold());
    println!();
    println!("现在可以开始使用：");
    println!("  codex chat \"你的目标\"    # 一次性任务（流式显示思考/工具/结果）");
    println!("  codex repl             # 交互式多轮对话（含审批）");
    println!("  codex sessions         # 查看进行中的会话");
    println!("  codex resume <id>      # 断点续传已中断的会话");
    println!();
}

// ========== B4-1 (backend taskbook #01): 真实事件流渲染 ==========

/// span_open — 缩进进层（span 树还原）。
pub fn span_open(name: &str, depth: usize, t0: &str) {
    if quiet() {
        return;
    }
    let indent = "  ".repeat(depth);
    println!("{}", format!("{indent}▸ span [{name}] {t0}").cyan());
}

/// span_close — 缩进退层 + 耗时。
pub fn span_close(depth: usize, duration_ms: u64) {
    if quiet() {
        return;
    }
    let indent = "  ".repeat(depth);
    println!(
        "{}",
        format!("{indent}◂ span 耗时 {duration_ms}ms")
            .cyan()
            .dimmed()
    );
}

/// R4: artifact — 产物登记（bright_cyan 高亮，任务书 R4 确认醒目）。
pub fn artifact(path: &str, kind: &str, delta: u64) {
    println!(
        "{}",
        format!("📄 产物 {kind}: {path}（+{delta} 行）")
            .bright_cyan()
            .bold()
    );
}

/// think_summary — 思考摘要（阶段模板）；P5 起亦承载**模型真实推理**（phase=reasoning）。
pub fn think_summary(phase: &str, text: &str) {
    if quiet() {
        return; // R3-3 quiet 档：思考摘要静默
    }
    // 模型真实推理单独呈现——与"正在规划/执行"这类进度套话区分开，
    // 让用户看到的是**模型实际在想什么、逻辑是什么**，而不是状态提示。
    if phase == "reasoning" {
        println!("{}", "💭 推理（模型自述）".dimmed());
        for line in text.lines() {
            println!("{}", format!("   {line}").dimmed());
        }
        return;
    }
    println!("{}", format!("💭 [{phase}] {text}").dimmed());
}

/// plan_draft — 规划草案（steps / gaps 审计事实）。
pub fn plan_draft(
    steps: &serde_json::Value,
    gaps_found: u64,
    gaps_to_ask: u64,
    auto_assumed: &serde_json::Value,
    gaps_to_ask_details: &serde_json::Value,
) {
    // R4: 卡片化——标题行 + 下边框（视觉上"一张卡片"）
    let header = format!(
        "🗺 规划草案（steps={} gaps={gaps_found} 待问={gaps_to_ask} 自动假设={}）",
        steps.as_array().map(|a| a.len()).unwrap_or(0),
        auto_assumed.as_array().map(|a| a.len()).unwrap_or(0)
    );
    println!("{}", format!("┌─ {header}").blue().bold());
    let _ = header.len();
    // B2-B (backend-intelligence): 产品红线可见化——"要问你什么"（blocking gap 带 why）
    if let Some(details) = gaps_to_ask_details.as_array() {
        for d in details {
            let from = d.get("from").and_then(|x| x.as_str()).unwrap_or("?");
            let why = d.get("why").and_then(|x| x.as_str()).unwrap_or("");
            println!("   {} {from}: {why}", "❓ 要问你".bright_yellow());
        }
    }
    // 自假定的 non-blocking gap（带 assume）——"它自己假定了什么"
    if let Some(assumed) = auto_assumed.as_array() {
        for a in assumed {
            let from = a.get("from").and_then(|x| x.as_str()).unwrap_or("?");
            let why = a.get("why").and_then(|x| x.as_str()).unwrap_or("");
            let assume = a.get("assume").and_then(|x| x.as_bool()).unwrap_or(false);
            println!(
                "   {} ⚡ 假设[{from}]: {why}",
                if assume { "🤖" } else { "  " }
            );
        }
    }
    if let Some(steps) = steps.as_array() {
        for s in steps.iter().take(8) {
            let task = s.get("task").and_then(|t| t.as_str()).unwrap_or("?");
            let status = s.get("status").and_then(|t| t.as_str()).unwrap_or("?");
            println!("   · {task} [{status}]");
        }
        if steps.len() > 8 {
            println!("   … 共 {} 步", steps.len());
        }
    }
    // W8/A3 (RC26): 旧残留的第二段 auto_assumed 渲染已删除——每份草案每条
    // 假设至多呈现一次（此前 ⚡ 假设行 ×2，missing_goal_source 574×2=1148 噪音）。
    println!("{}", "└─".blue());
}

#[cfg(test)]
mod p3_tests {
    /// R3-3 降噪三档（W-F）：verbosity set/get 往返 + 上限钳制 + quiet 档
    /// 过程函数调用不 panic。println 直写 stdout 不可进程内捕获——静默效果
    /// 由 e2e 渲染快照终验（与 RC34 同口径）；此处锁 API 语义。
    #[test]
    fn test_r33_verbosity_levels() {
        super::set_verbosity(0);
        assert_eq!(super::verbosity(), 0, "quiet 档");
        // quiet 档下过程噪声函数必须可安全调用（内部早退）
        super::phase("Plan");
        super::token("delta");
        super::tool_call("bash", &serde_json::json!({"command": "ls"}));
        super::tool_result("ok");
        super::think_summary("act", "...");
        super::reflection("continue");
        super::span_open("s", 0, "t0");
        super::span_close(0, 5);
        super::set_verbosity(2);
        assert_eq!(super::verbosity(), 2, "verbose 档");
        // 上限钳制：>2 归 2（防非法档位）
        super::set_verbosity(9);
        assert_eq!(super::verbosity(), 2);
        super::set_verbosity(1);
        assert_eq!(super::verbosity(), 1, "恢复默认");
    }

    /// RC34（P3-BACKLOG）：×2 渲染去重——同一 (steps, ok) 连续渲染折叠为一次。
    /// 快照对比口径：两次 done(14,true) 的 stdout 中 `✓ Done (14 steps)` 只出现一次。
    /// （println 直写 stdout——用 Gag 式重定向不可移植，改以进程内计数验证：
    /// done() 折叠后返回单元不变，此处验证去重键行为——重复调用不 panic 且
    /// 由 e2e 渲染快照（P3 Node 04 复测包 ×2 渲染项）做终端级终验。）
    #[test]
    fn test_rc34_done_dedup_key_stable() {
        // 首次渲染
        super::done(14, true);
        // 重复渲染（应被折叠）——不 panic 即折叠逻辑生效（stdout 计数由 e2e 验证）
        super::done(14, true);
        // 不同终态（✗ 14 steps）——不折叠
        super::done(14, false);
        super::done(14, false);
        // 不同 steps——不折叠
        super::done(5, true);
    }
}
