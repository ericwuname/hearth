use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sandbox::{Sandbox, SandboxConfig, SandboxOutput};
use std::sync::Arc;
use std::time::Duration;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct BashTool {
    sandbox: Arc<dyn Sandbox>,
}

impl BashTool {
    pub fn new() -> Self {
        Self {
            // v12.4: bash is the tool agents use to run `cargo test` / `npm test`.
            // The default probe profile (30s, 512MB, cwd-only writes) killed every
            // build, so use the build-tools profile.
            sandbox: Arc::from(sandbox::create_sandbox(SandboxConfig::for_build_tools())),
        }
    }

    pub fn with_sandbox(sandbox: Arc<dyn Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

/// WS5 (v0.1.5): 危险命令检测——命中返回原因，None=放行。
/// 命令级确定性护栏（沙箱 seccomp 拦 syscall、此层拦高危命令模式）：
/// 根删除 / 格式化 / 系统关闭 / fork bomb / 全盘权限 / 写块设备。
/// 精确匹配防误伤：`rm -rf ./build`、`rm -rf /home/x/tmp` 等合法清理不受影响。
/// P5 · S5-② 出网白名单检查（纯函数，可单测）。
///
/// 沙箱 seccomp 管 syscall、管不了域名；域名级收口在工具层做。
/// 判定（全部确定性，不猜意图）：
///   1. 命令既不含 URL、也不含已知网络命令 → 放行（普通命令不受影响）；
///   2. 含网络意图 → 提取主机，逐个比对白名单（相等或以 `.条目` 结尾）；
///   3. **白名单为空 = 全拒**（与 web_fetch 的 T10 口径一致：空=全拒）。
///   4. 提取不到任何主机但确属网络命令 → 拒绝（fail-closed，不放行裸 `curl` 到未知目标）。
pub fn check_egress(
    cmd: &str,
    env: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    const NET_CMDS: [&str; 9] = [
        "curl ", "wget ", "nc ", "ncat ", "ssh ", "scp ", "rsync ", "ftp ", "sftp ",
    ];
    let lower = cmd.to_ascii_lowercase();
    let has_url = lower.contains("http://") || lower.contains("https://");
    let has_net_cmd = NET_CMDS.iter().any(|c| lower.contains(c));
    if !has_url && !has_net_cmd {
        return Ok(());
    }

    let allow_csv = env
        .get("HEARTH_EGRESS_ALLOWLIST")
        .cloned()
        .unwrap_or_default();
    let allow: Vec<&str> = allow_csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let hosts = extract_hosts(cmd);
    if allow.is_empty() {
        return Err(format!(
            "出网被拒：egress_allowlist 为空（空=全拒）。目标 {:?} 未获放行。\
             请在 config.toml 的 egress_allowlist 中补入需要的域名。",
            hosts
        ));
    }
    if hosts.is_empty() {
        return Err(format!(
            "出网被拒：命令含网络调用但无法识别目标主机（fail-closed）。\
             请使用完整 URL（如 https://example.com/...）以便按白名单校验。当前白名单 {} 条。",
            allow.len()
        ));
    }
    let denied: Vec<String> = hosts
        .iter()
        .filter(|h| {
            !allow
                .iter()
                .any(|a| *a == h.as_str() || h.ends_with(&format!(".{a}")))
        })
        .cloned()
        .collect();
    if denied.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "出网被拒：目标域名 {:?} 不在 egress_allowlist 白名单（{} 条）内。\
                 需要放行请让用户在 config.toml 的 egress_allowlist 中补入该域名。",
            denied,
            allow.len()
        ))
    }
}

/// 从命令中提取主机名：`https://host/path`、`http://host`、以及 bare host 形式。
/// 纯字符串处理，不做 DNS（保持确定性与零副作用）。
///
/// 误报修复（v0.2.22 复核实测）：
///   `curl https://example.com > VERIFY.txt` 曾把 **`verify.txt`**（重定向目标
///   文件名）当成裸主机 → 整条命令被 fail-closed 拒绝，连白名单内的目标也被
///   连坐。修法两点：
///   ① **位置感知**：紧跟在 `>` / `>>` / `2>` / `&>` / `<` 等重定向符之后的
///      token 是文件目标，永不视为主机；
///   ② **文件扩展名排除**：末段为常见数据/代码文件扩展名的 token 不视为主机
///      （`.rs/.toml` 的老排除并入此表——逐个加后缀是打地鼠，一次收口）。
fn extract_hosts(cmd: &str) -> Vec<String> {
    /// 常见**文件**扩展名——这些末段几乎不可能是真实 TLD，出现即视为文件名。
    const FILE_EXTS: [&str; 26] = [
        "txt", "json", "jsonl", "log", "md", "csv", "yaml", "yml", "xml", "html", "htm", "py",
        "js", "ts", "sh", "out", "err", "cfg", "conf", "ini", "tar", "gz", "zip", "png", "jpg",
        "pdf",
    ];
    /// 重定向操作符——其后一个 token 是文件目标，不是网络主机。
    const REDIRECT_OPS: [&str; 9] = [">", ">>", "2>", "2>>", "&>", "&>>", "<", "1>", "1>>"];

    let mut out: Vec<String> = Vec::new();
    let mut prev_op = false; // 上一个 token 是否为重定向符
    for token in cmd.split(|c: char| c.is_whitespace() || c == '\'' || c == '"') {
        let t = token.trim_matches(|c: char| c == ',' || c == ';' || c == ')' || c == '(');

        // 记录本 token 是否为重定向符（供下一个 token 判断）
        let is_op = REDIRECT_OPS.contains(&t);

        if prev_op {
            // 重定向目标：文件路径，永不视为主机
            prev_op = is_op; // 连续重定向符（如 `2>>` 拆分后）仍按符处理
            continue;
        }

        if let Some(rest) = t
            .strip_prefix("https://")
            .or_else(|| t.strip_prefix("http://"))
        {
            if let Some(host) = rest.split(['/', ':', '?', '#']).next() {
                if !host.is_empty() && host.contains('.') {
                    let h = host.to_ascii_lowercase();
                    if !out.contains(&h) {
                        out.push(h);
                    }
                }
            }
        } else if t.contains('.')
            && !t.starts_with('-')
            && !t.starts_with('/')
            && !t.contains('=')
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        {
            // 形如 `example.com` 的裸主机（curl example.com）
            let h = t.to_ascii_lowercase();
            let last_seg = h.rsplit('.').next().unwrap_or("");
            if h.split('.').count() >= 2 && !FILE_EXTS.contains(&last_seg) && !out.contains(&h) {
                out.push(h);
            }
        }

        prev_op = is_op;
    }
    out
}

fn dangerous_command(cmd: &str) -> Option<&'static str> {
    let c = cmd.trim();
    // 根目录删除：rm -rf / 后跟 空格/引号/结尾（限定路径的 rm -rf 不拦）
    if c == "rm -rf /" {
        return Some("rm -rf /（删除根目录）");
    }
    for p in ["rm -rf / ", "rm -rf /'", "rm -rf /\""] {
        if c.starts_with(p) {
            return Some("rm -rf /（删除根目录）");
        }
    }
    if c.starts_with("rm -rf /*") {
        return Some("rm -rf /*（通配删除根目录）");
    }
    // 格式化磁盘
    if c.contains("mkfs") {
        return Some("mkfs（格式化磁盘）");
    }
    // 系统关闭/重启（独立命令，词边界）
    for w in ["shutdown", "reboot", "halt", "poweroff"] {
        if c == w || c.starts_with(&format!("{w} ")) || c.starts_with(&format!("{w} -")) {
            return Some("系统关闭/重启命令");
        }
    }
    // fork bomb
    if c.contains(":(){") || c.contains(":() {") {
        return Some("fork bomb");
    }
    // 全盘权限
    if c.contains("chmod -R 777 /") || c.contains("chmod -R 777 /*") {
        return Some("全盘 chmod 777");
    }
    // 写块设备
    if c.contains("of=/dev/sd")
        || c.contains("of=/dev/nvme")
        || c.contains("> /dev/sd")
        || c.contains("> /dev/nvme")
    {
        return Some("写入块设备");
    }
    None
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    /// H1 (v0.2.4): dispatcher 层时间窗须容下内部最大 timeout_secs（600s），
    /// 否则外层 30s 总闸先把长跑命令杀掉（手工实测 cargo build 反复被灭）。
    fn declared_timeout(&self) -> Duration {
        Duration::from_secs(610)
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "bash".into(),
            description: "Execute a bash command in an isolated sandbox.\n\
                 何时用: 运行测试/编译/脚本（cargo test、python x.py）、查看环境（ls/cat/pwd）、\n\
                 以及修改代码后验证（关键：验证命令非零退出 = 失败，不得声称完成）。\n\
                 何时不用: 读单个文件（read 更省且带行号）；找文件/内容（glob/grep）；写文件（write_file）。\n\
                 示例: bash(\"cargo test 2>&1 | tail -30\")；bash(\"python game.py < /dev/null | head -20\")。\n\
                 边界: 每命令默认 timeout 180s（max 600）；出网仅限白名单域名；高危命令（rm -rf 等）被护栏拦截；\n\
                 stdout+stderr 截断后仅保留首尾段——长输出用 tail/head/grep 收敛。\n\
                 错误解读: 返回含 \"exit code: N\"（N≠0 = 命令失败，读 stdout/stderr 定位原因）；\n\
                 \"command blocked\"=触犯安全护栏，换安全写法；\"egress denied\"=域名不在白名单。"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "cmd": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Optional timeout in seconds (default 180, max 600)"
                    }
                },
                "required": ["cmd"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let cmd = args
            .get("cmd")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing 'cmd' argument"))?;

        // P5-FOUNDATION-01 · S5-② 出网白名单（顶层裁决：产品可联网，但必须按白名单走）。
        // 沙箱的 seccomp 只能管 syscall，管不了域名——域名级收口必须在工具层做。
        // 口径：命令中出现 URL 或已知网络命令 → 提取主机 → 不在白名单即确定性拒绝
        // （不猜、不降级放行；白名单由 config.toml 的 egress_allowlist 注入 ctx.env）。
        if let Err(why) = check_egress(cmd, &ctx.env) {
            return Err(anyhow!("{why}"));
        }

        // WS5 (v0.1.5): 危险命令确定性护栏（Claude Code 架构签名对齐）——
        // 命令级拦截沙箱 syscall 拦不到的高危模式（根删除/格式化/关机/fork bomb）。
        // 命中即拒绝（fail-closed），不静默放行。合法清理（rm -rf ./build）不受影响。
        if let Some(why) = dangerous_command(cmd) {
            return Err(anyhow!(
                "危险命令拦截: {why}——hearth 拒绝执行破坏性/系统级命令。如需清理请限定明确路径（如 rm -rf ./build）。"
            ));
        }
        // RC53 (P4 Node 06): 内部工具名被序列化进 bash 命令（盲测 exit 127 实证）——
        // 结构化提示引导模型改用工具调用。边界 = 已注册**内部工具名**（非关键词黑名单）。
        const INTERNAL_TOOL_NAMES: &[&str] = &["introspect"];
        let first_word = cmd.split_whitespace().next().unwrap_or("");
        if INTERNAL_TOOL_NAMES.contains(&first_word) {
            return Err(anyhow!(
                "`{first_word}` 是 hearth 内部工具，不是 shell 命令——请直接发起工具调用（tool: {first_word}），不要通过 bash 执行。"
            ));
        }

        // v12.4: 30s was not enough to compile+test even a trivial Rust crate,
        // so every "cargo test" step timed out. Default to 180s, cap at 600s.
        // P1-LTR-01 Phase 4: dispatcher 单点已按任务剩余期收紧（ctx.effective_timeout
        // = min(declared, remaining)）——bash 优先消费本值，sandbox.spawn 按真实
        // 剩余时间终止（对照旧实证：sleep 300 + task deadline 15s 穿透 360s）。
        // 无 deadline（None）→ 旧 args 语义（180 默认/600 cap）分毫不变。
        let timeout_secs = ctx
            .effective_timeout
            .map(|d| d.as_secs())
            .unwrap_or_else(|| {
                args.get("timeout_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(180)
                    .min(600)
            });

        let mut env: Vec<(&str, String)> = ctx
            .env
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_string()))
            .collect();

        // P3 (v24-post): 注入 Rust 工具链 PATH——agent 在 sandbox 内自测必需。
        // 此前 sandbox 内 `cargo`/`rustc` 不在 PATH（"cargo: 未找到命令"），agent
        // 把步数耗在修环境上、无法自测改动（10-run 实录 9/10 budget exhausted）。
        // 工具层注入：不碰 G0 sandbox crate、不污染 goal 文本。
        if let Some(home) = std::env::var_os("HOME") {
            let cargo_bin = std::path::PathBuf::from(&home).join(".cargo/bin");
            let mut found = false;
            for (k, v) in env.iter_mut() {
                if *k == "PATH" {
                    *v = format!("{}:{}", cargo_bin.display(), v);
                    found = true;
                    break;
                }
            }
            if !found {
                // 原 PATH 缺失时保留系统默认路径——否则 bash 自身都找不到
                env.push((
                    "PATH",
                    format!("{}:/usr/local/bin:/usr/bin:/bin", cargo_bin.display()),
                ));
            }
        }
        let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();

        let output = self
            .sandbox
            .spawn(
                "bash",
                &["-c", cmd],
                &ctx.cwd,
                &env_refs,
                Duration::from_secs(timeout_secs),
            )
            .await?;

        // W4/RC20: 非零退出码/超时 → Err（结构化 error）——scheduler 据此置
        // ToolResult.is_error=true，CLI/渲染层消费结构化字段，不再猜字符串
        // （中文错误输出正确投影失败；正常输出含 error 词不再误画红 ✗）。
        let timed_out = output.timed_out;
        let exit_code = output.exit_code;
        let formatted = format_output(output);
        if timed_out || exit_code != 0 {
            // P2-LR Node 02（RC46 接线）：类型化退出状态（非文本）——scheduler
            // downcast 投影 error_kind（exit >0 / signal / tool timeout 可区分）。
            return Err(anyhow::Error::new(tool_runtime::BashExitError {
                exit_code,
                timed_out,
                formatted,
            }));
        }
        Ok(formatted)
    }
}

fn format_output(out: SandboxOutput) -> String {
    if out.timed_out {
        return format!(
            "error: command timed out\nstdout:\n{}\nstderr:\n{}",
            out.stdout, out.stderr
        );
    }
    let base = if out.exit_code == 0 {
        out.stdout.clone()
    } else {
        format!(
            "exit code: {}\nstdout:\n{}\nstderr:\n{}",
            out.exit_code, out.stdout, out.stderr
        )
    };
    // read_lints (v0.2): cargo/rustc 编译错误结构化提取——agent 一眼看到"哪行错了"
    // （不再只给原始输出让它自己 parse）。非编译输出则原样返回。
    let lints = extract_lints(&base);
    if lints.is_empty() {
        base
    } else {
        format!("{base}\n{lints}")
    }
}

/// read_lints (v0.2): 从编译输出提取结构化错误（error[EXXXX] + 文件:行:列 + 消息）。
/// 返回 `[lints] ...` 段；无编译错误返回空串（不干扰正常输出）。
fn extract_lints(text: &str) -> String {
    #[derive(Default)]
    struct Err3 {
        code: String,
        loc: String,
        msg: String,
    }
    let mut errs: Vec<Err3> = Vec::new();
    let mut cur: Option<Err3> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("error[") {
            // 新错误开始——闭合上一个
            if let Some(e) = cur.take() {
                errs.push(e);
            }
            if let Some(end) = rest.find("]:") {
                let mut e = Err3 {
                    code: rest[..end].to_string(),
                    loc: String::new(),
                    msg: rest[end + 2..].trim().to_string(),
                };
                e.code = e
                    .code
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .collect();
                cur = Some(e);
            }
        } else if t.starts_with("error: ") && cur.is_none() {
            cur = Some(Err3 {
                code: String::new(),
                loc: String::new(),
                msg: t[7..].to_string(),
            });
        } else if let Some(loc_part) = t.strip_prefix("--> ") {
            if let Some(e) = cur.as_mut() {
                if e.loc.is_empty() {
                    e.loc = loc_part.to_string();
                }
            }
        } else if (t.starts_with("error") || t.starts_with("warning"))
            && cur.is_some()
            && !t.starts_with("error[")
            && !t.starts_with("error: ")
        {
            // 下一个错误/警告标题——闭合当前
            if let Some(e) = cur.take() {
                errs.push(e);
            }
        }
    }
    if let Some(e) = cur.take() {
        errs.push(e);
    }
    if errs.is_empty() {
        return String::new();
    }
    let mut out = String::from("[lints] 编译错误结构化提取（read_lints）:\n");
    for (i, e) in errs.iter().enumerate() {
        let code_part = if e.code.is_empty() {
            "error".to_string()
        } else {
            format!("E{}", e.code)
        };
        let loc_part = if e.loc.is_empty() {
            String::new()
        } else {
            format!(" @ {}", e.loc)
        };
        let msg_part = if e.msg.is_empty() {
            String::new()
        } else {
            format!(" — {}", e.msg)
        };
        out.push_str(&format!("  {}) {code_part}{loc_part}{msg_part}\n", i + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tool_runtime::ToolContext;

    fn env_with_allowlist(list: &str) -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("HEARTH_EGRESS_ALLOWLIST".to_string(), list.to_string());
        m
    }

    /// P5 S5-②：白名单内域名放行（含子域）。
    #[test]
    fn test_egress_allows_allowlisted_host() {
        let env = env_with_allowlist("example.com,crates.io");
        assert!(check_egress("curl https://example.com/", &env).is_ok());
        assert!(check_egress("curl https://index.crates.io/config.json", &env).is_ok());
        assert!(check_egress("echo hello", &env).is_ok(), "普通命令不受影响");
    }

    /// P5 S5-②：非白名单域名必须被拒（先红后绿：删掉拒绝逻辑此断言必红）。
    #[test]
    fn test_egress_denies_unlisted_host() {
        let env = env_with_allowlist("example.com");
        let e = check_egress("curl https://evil.test/", &env).unwrap_err();
        assert!(e.contains("不在 egress_allowlist"), "got: {e}");
        assert!(e.contains("evil.test"), "须点名被拒域名: {e}");
    }

    /// 空白名单 = 全拒（与 web_fetch T10 口径一致）。
    #[test]
    fn test_egress_empty_allowlist_denies_all() {
        let env = env_with_allowlist("");
        assert!(check_egress("curl https://example.com/", &env).is_err());
        let env2 = std::collections::HashMap::new();
        assert!(check_egress("curl https://example.com/", &env2).is_err());
    }

    /// 网络命令但识别不出目标主机 → fail-closed 拒绝。
    #[test]
    fn test_egress_unresolved_target_denied() {
        let env = env_with_allowlist("example.com");
        let e = check_egress("curl -s -o /dev/null -w %{http_code} http://", &env).unwrap_err();
        assert!(e.contains("无法识别目标主机"), "got: {e}");
    }

    /// v0.2.22 复核实测误报修复（先红后绿）：
    /// `> VERIFY.txt` 这类**重定向目标文件名**曾被 extract_hosts 当成裸主机
    /// （verify.txt 有点、两段、不在旧排除表）→ 整条命令 fail-closed 连坐，
    /// 白名单内的 example.com 也被拒（执行窗复核探针实测 2 次误拒 + budget 耗尽）。
    #[test]
    fn test_egress_redirect_target_not_a_host() {
        let env = env_with_allowlist("example.com");
        // 复现原样：块重定向 + 2>&1
        assert!(check_egress(
            "curl -s -o /dev/null -w \"code=%{http_code}\" --max-time 20 https://example.com > VERIFY.txt 2>&1",
            &env
        ).is_ok(), "重定向目标文件名不得当主机");
        // 小写/常见名同样不得当主机
        assert!(check_egress(
            "curl https://example.com > out.txt && curl https://example.com >> out.txt",
            &env
        )
        .is_ok());
        // 文件扩展名排除：-o out.json / 2> err.log
        assert!(check_egress("curl https://example.com -o out.json 2> err.log", &env).is_ok());
        // 真正的非白名单域仍须被拒（防修复矫枉过正）
        let e = check_egress("curl https://www.baidu.com > baidu.txt", &env).unwrap_err();
        assert!(e.contains("baidu.com"), "got: {e}");
        assert!(!e.contains("baidu.txt"), "文件名不得出现在被拒清单: {e}");
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(serde_json::json!({"cmd": "echo hello"}), &ctx)
            .await
            .unwrap();
        assert!(result.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_missing_cmd() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        let result = tool.execute(serde_json::json!({}), &ctx).await;
        assert!(result.is_err());
    }

    /// W4/RC20（红→绿）: 非零退出码 → Err（结构化 error——scheduler 据此置
    /// ToolResult.is_error=true）。修复前返回 Ok("exit code: 1...") → is_error=false
    /// → 中文/任意错误输出被渲染层猜字符串误画绿 ✓。错误消息必须保留退出码与输出。
    #[tokio::test]
    async fn test_bash_failure_is_structured_error() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(serde_json::json!({"cmd": "exit 1"}), &ctx)
            .await;
        assert!(
            result.is_err(),
            "非零退出码必须 Err（结构化 error）——修复前 Ok 导致 is_error=false 假绿"
        );
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("exit code: 1"),
            "错误消息必须保留退出码与输出: {msg}"
        );
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(
                serde_json::json!({"cmd": "sleep 10", "timeout_secs": 1}),
                &ctx,
            )
            .await;
        // W4/RC20: 超时同样走结构化 Err
        assert!(result.is_err(), "超时必须 Err（结构化 error）");
        assert!(
            format!("{}", result.unwrap_err()).contains("timed out"),
            "超时消息必须保留"
        );
    }

    /// 回归 R4 (v0.1.3 B6 + v0.1.4 补测): 验证类命令返回真实退出码——
    /// node --check 曾被 seccomp 缺 syscall（epoll_wait 等）KILL 成 exit -1。
    /// Linux 真 sandbox 下 node 必须能完成语法检查（真实退出码 0/1）；
    /// 白名单退化（106 旧表）时此测试红。Windows NoopSandbox 不拦（pass 无害）。
    #[tokio::test]
    async fn test_bash_node_check_real_exit_code() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(
                serde_json::json!({"cmd": "echo 'const x = 1;' > /tmp/h_verify.js && node --check /tmp/h_verify.js; echo RC=$?"}),
                &ctx,
            )
            .await
            .unwrap();
        // 真实退出码 0（node 语法 OK）——不允许 exit code: -1（信号杀）。
        assert!(
            !result.contains("exit code: -1"),
            "node --check 被沙箱信号杀（seccomp 缺 syscall），got: {result}"
        );
        assert!(
            result.contains("RC=0"),
            "node --check 应返回真实退出码 0，got: {result}"
        );
    }

    /// 回归 R4 (v0.1.4 补测): tail 管道读取正常（v0.1.3 曾怀疑被白名单拦截）。
    #[tokio::test]
    async fn test_bash_tail_normal_output() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(serde_json::json!({"cmd": "seq 1 10 | tail -3"}), &ctx)
            .await
            .unwrap();
        assert!(
            !result.contains("exit code: -1"),
            "tail 被沙箱信号杀，got: {result}"
        );
        assert!(
            result.contains("10"),
            "tail -3 应输出末尾行（含 10），got: {result}"
        );
    }

    /// 回归 WS5 (v0.1.5): 危险命令确定性护栏——破坏性/系统级命令拒绝，
    /// 合法限定路径清理放行（防误伤）。无护栏时此测试红。
    #[tokio::test]
    async fn test_bash_dangerous_command_blocked() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        for (bad, what) in [
            ("rm -rf /", "根删除"),
            ("rm -rf /*", "通配根删除"),
            ("rm -rf / tmp", "根删除（后接空格）"),
            ("shutdown -h now", "关机"),
            ("reboot", "重启"),
            ("mkfs.ext4 /dev/sdb1", "格式化"),
            ("echo x | :(){ :|:& };:", "fork bomb"),
            ("chmod -R 777 /", "全盘权限"),
        ] {
            let r = tool.execute(serde_json::json!({"cmd": bad}), &ctx).await;
            assert!(r.is_err(), "{what} 必须被拦截: {bad}");
            assert!(
                r.unwrap_err().to_string().contains("危险命令拦截"),
                "错误须含'危险命令拦截'"
            );
        }
    }

    /// 回归 WS5 (v0.1.5): 合法清理/常规命令不得误伤（防过度拦截）。
    #[tokio::test]
    async fn test_bash_dangerous_not_false_positive() {
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        for (ok, what) in [
            ("rm -rf ./build", "限定路径清理"),
            ("rm -rf /home/wutao/codex/target", "限定路径清理"),
            ("echo shutdown", "文本含 shutdown 但非命令"),
            ("ls /tmp", "常规命令"),
            ("cargo test 2>&1 | tail -30", "构建验证"),
            (
                "dd if=/dev/zero of=/tmp/zero.bin bs=1M count=1",
                "写入临时文件（非块设备）",
            ),
        ] {
            let r = tool.execute(serde_json::json!({"cmd": ok}), &ctx).await;
            assert!(
                !r.as_ref()
                    .is_err_and(|e| e.to_string().contains("危险命令拦截")),
                "{what} 不应被误拦: {ok} -> {r:?}"
            );
        }
    }

    #[test]
    fn test_description() {
        let tool = BashTool::new();
        let desc = tool.description();
        assert_eq!(desc.name, "bash");
    }
}

/// 回归 read_lints (v0.2): cargo 编译错误输出 → 结构化提取（code+位置+消息）；
/// 正常输出不追加 lints 段（防噪音）。无提取时此测试红。
#[test]
fn test_extract_lints_struct() {
    let out = "\
error[E0308]: mismatched types
  --> src/main.rs:12:9
   |
12 |     let x: u32 = \"hello\";
   |         ^ expected `u32`, found `&str`
error[E0433]: failed to resolve: use of undeclared crate or module `foo`
  --> src/lib.rs:20:5
   |
20 |     foo::bar()
   |     ^^^ use of undeclared crate or module `foo`
";
    let lints = extract_lints(out);
    assert!(lints.contains("[lints]"), "应输出 lints 段: {lints}");
    assert!(lints.contains("E0308"), "应含错误码 E0308");
    assert!(lints.contains("src/main.rs:12:9"), "应含位置");
    assert!(lints.contains("mismatched types"), "应含错误消息");
    assert!(lints.contains("E0433"));
    assert!(lints.contains("src/lib.rs:20:5"));

    // 正常输出（无编译错误）不追加
    assert!(extract_lints("test result: ok. 10 passed").is_empty());
    assert!(extract_lints("hello world\n").is_empty());
}
