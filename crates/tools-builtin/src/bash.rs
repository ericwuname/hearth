use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sandbox::{Sandbox, SandboxConfig, SandboxOutput};
use std::sync::Arc;
use std::time::Duration;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct BashTool {
    sandbox: Arc<dyn Sandbox>,
}

/// S15（手术包二）：bash 会话 cwd 持久化——多步构建/长任务不再每次重敲路径。
/// 进程级单例（hearth = 单进程跑一个会话）；`HEARTH_BASH_NO_PERSIST=1` 关闭
/// （测试隔离用）。首次调用以 ToolContext.cwd 为起点，之后跟随命令内 cd。
static SESSION_CWD: std::sync::OnceLock<std::sync::Mutex<Option<std::path::PathBuf>>> =
    std::sync::OnceLock::new();

/// 会话 cwd 标记（钉死格式——与 [net] 审计同风格，LLM 可见）。
pub const CWD_MARKER: &str = "__HEARTH_CWD__=";

fn session_cwd_slot() -> &'static std::sync::Mutex<Option<std::path::PathBuf>> {
    SESSION_CWD.get_or_init(|| std::sync::Mutex::new(None))
}

/// S15: 持久化开关（默认开；HEARTH_BASH_NO_PERSIST=1 关闭——测试/隔离场景）。
pub fn persist_enabled() -> bool {
    !matches!(std::env::var("HEARTH_BASH_NO_PERSIST"), Ok(v) if v == "1")
}

/// S15: 记录会话 cwd（执行后回写；测试可直接调用做隔离）。
pub fn set_session_cwd(p: std::path::PathBuf) {
    if let Ok(mut g) = session_cwd_slot().lock() {
        *g = Some(p);
    }
}

/// S15: 读取会话 cwd（无则 None）。
pub fn get_session_cwd() -> Option<std::path::PathBuf> {
    session_cwd_slot().lock().ok().and_then(|g| g.clone())
}

/// S15: 命令包装——先 cd 会话 cwd（若有），执行用户命令，尾部打印新 cwd 标记，
/// 保持用户命令退出码。用户命令的多行/&&/||语义由 `{ ...; }` 保留。
pub fn wrap_with_cwd(cmd: &str, base: &std::path::Path, already: Option<&std::path::Path>) -> String {
    let start = already.unwrap_or(base);
    let start_disp = start.display();
    // cd 失败不阻断（路径消失时退回原 cwd 执行，标记仍会回写真实 PWD）。
    format!(
        "cd '{start_disp}' 2>/dev/null || true\n{{ {cmd}\n}}\n__hearth_rc=$?\nprintf '\\n{s}{{%s}}\\n' \"$PWD\"\nexit $__hearth_rc",
        s = CWD_MARKER
    )
}

/// S15: 从输出中剥离 cwd 标记并返回（新 cwd, 干净输出）。
/// 标记形如 `__HEARTH_CWD__={/path}`（花括号防路径含空格歧义）。
/// **只采纳最后一个标记行**（包装注入必然在输出末尾；命令回显的早期标记行保留，
/// 防模型 echo 伪造 cwd 改变会话状态）。
pub fn extract_cwd_marker(formatted: &str) -> (Option<std::path::PathBuf>, String) {
    let lines: Vec<&str> = formatted.lines().collect();
    // 从尾部找第一个含标记且可解析出非空路径的行——即"最后一个有效标记行"。
    let mut hit: Option<usize> = None;
    let mut new_cwd: Option<std::path::PathBuf> = None;
    for (i, line) in lines.iter().enumerate().rev() {
        if let Some(pos) = line.find(CWD_MARKER) {
            let raw = line[pos + CWD_MARKER.len()..].trim();
            let raw = raw
                .strip_prefix('{')
                .and_then(|r| r.strip_suffix('}'))
                .unwrap_or(raw);
            if !raw.is_empty() {
                hit = Some(i);
                new_cwd = Some(std::path::PathBuf::from(raw));
                break;
            }
        }
    }
    match hit {
        Some(i) => {
            let kept: Vec<&str> = lines
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, l)| *l)
                .collect();
            (new_cwd, kept.join("\n"))
        }
        None => (None, formatted.to_string()),
    }
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
    // hearth-slim S2（用户拍板 2026-09-09"全部默认允许"，顶层落字备案）：
    // 语义反转——白名单空/未设 = 默认放开；显式 allowlist 仍收紧（fail-closed
    // 只在收紧模式内）。风险（提示注入/恶意命令）已知悉接受，审计投影为
    // 唯一缓解层（见 execute 尾部 [net] 行，格式钉死见测试注释）。
    if allow.is_empty() {
        return Ok(());
    }
    if hosts.is_empty() {
        return Err(format!(
            "出网被拒：命令含网络调用但无法识别目标主机（收紧模式 fail-closed）。\
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

/// hearth-slim S1: 超时解析纯函数（可单测）——
/// 优先级：任务剩余期（effective，deadline 收口）> args.timeout_secs >
/// env HEARTH_TOOL_TIMEOUT_SECS（config 注入或进程级）> 默认 120；
/// 上限 600 不变。默认从 180 收到 120（C-fix-status 挂死 5.5h 直接动因：
/// 无 deadline 时长默认暴露面过大）。
fn resolve_timeout_secs(
    args: &serde_json::Value,
    effective: Option<Duration>,
    env: &std::collections::HashMap<String, String>,
) -> u64 {
    let cap = |s: u64| s.min(600);
    if let Some(d) = effective {
        return cap(d.as_secs());
    }
    if let Some(v) = args.get("timeout_secs").and_then(|x| x.as_u64()) {
        return cap(v);
    }
    let from_env = env
        .get("HEARTH_TOOL_TIMEOUT_SECS")
        .and_then(|s| s.trim().parse::<u64>().ok())
        .or_else(|| {
            std::env::var("HEARTH_TOOL_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
        });
    cap(from_env.unwrap_or(120))
}

/// hearth-slim S1: 输出截断（工具层，sandbox crate 零触碰）——
/// stdout/stderr 任一超 64KB → 保头 2/3 + 尾 1/3，中间显式标记（防
/// 45 万 tokens 式输出膨胀复发；`[truncated]` 标记供模型/测试识别）。
const S1_MAX_OUTPUT_BYTES: usize = 64 * 1024;

fn truncate_output(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let head = max * 2 / 3;
    let tail = max / 3;
    let mut h = head;
    while h > 0 && !s.is_char_boundary(h) {
        h -= 1;
    }
    let mut tl = s.len() - tail;
    while tl < s.len() && !s.is_char_boundary(tl) {
        tl += 1;
    }
    let dropped = s.len() - h - (s.len() - tl);
    format!(
        "{}\n[truncated] …输出超 64KB，中间 {} bytes 已截断（hearth-slim S1 防膨胀）…\n{}",
        &s[..h],
        dropped,
        &s[tl..]
    )
}

/// hearth-slim S2: 出网审计投影的数据源——与 check_egress 同一意图判定
/// （URL 或已知网络命令），返回提取到的主机清单。非网络命令 → None。
/// 审计行格式（顶层附加要求：落刀前钉死）：
///   `[net] egress audit: <host1> <host2> …`；目标不可解析时
///   `[net] egress audit: target unresolved`。单行、尾部追加、不阻断不解析。
pub fn net_audit_hosts(cmd: &str) -> Option<Vec<String>> {
    const NET_CMDS: [&str; 9] = [
        "curl ", "wget ", "nc ", "ncat ", "ssh ", "scp ", "rsync ", "ftp ", "sftp ",
    ];
    let lower = cmd.to_ascii_lowercase();
    let has_url = lower.contains("http://") || lower.contains("https://");
    let has_net_cmd = NET_CMDS.iter().any(|c| lower.contains(c));
    if !has_url && !has_net_cmd {
        return None;
    }
    Some(extract_hosts(cmd))
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
                 边界: 每命令默认 timeout 120s（max 600；env HEARTH_TOOL_TIMEOUT_SECS 可配）；出网默认放开（网络活动带 [net] 审计投影；HEARTH_EGRESS_ALLOWLIST 可收紧）；高危命令（rm -rf 等）被护栏拦截；\n\
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
                        "description": "Optional timeout in seconds (default 120, max 600)"
                    }
                },
                "required": ["cmd"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        // K-3（契约手术，2026-09-11）：参数归一契约——与 read/grep/edit 同款
        // extract_str_arg（LLM 层 JSON 降级为 raw String 时，裸 `.get("cmd")` 必
        // 失败 → "missing 'cmd'"、命令全废；EMBER M1 施工实证同类病灶的破坏力）。
        // 先红后绿证据：test_k3_cmd_arg_three_shapes——回退版 FAILED（②raw-String
        // missing 'cmd'）/修复版 ok。
        let cmd = crate::extract_str_arg(&args, "cmd")
            .ok_or_else(|| anyhow!("missing 'cmd' argument"))?;
        let cmd: &str = cmd.as_str();

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
        let timeout_secs = resolve_timeout_secs(&args, ctx.effective_timeout, &ctx.env);

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

        // S1: elapsed 计时（结构化 timeout 载荷用）。
        let t0 = std::time::Instant::now();
        // S1: 沙箱双后端超时行为统一收口——NoopSandbox（非 Linux 开发态）超时
        // 返回 Err("command timed out after …")，LinuxSandbox 返回
        // Ok(SandboxOutput{timed_out:true})。两条路径都归一到本层的结构化
        // JSON（sandbox crate 零触碰）。
        let s1_timeout_payload = |elapsed: u64, limit: u64, stdout: &str, stderr: &str| {
            let partial = format!(
                "stdout:\n{}\nstderr:\n{}",
                truncate_output(stdout, S1_MAX_OUTPUT_BYTES),
                truncate_output(stderr, S1_MAX_OUTPUT_BYTES / 4),
            );
            serde_json::json!({
                "status": "timeout",
                "elapsed_secs": elapsed,
                "timeout_limit_secs": limit,
                "partial_output": partial,
            })
            .to_string()
        };
        // S1 支撑（问题卡预防：Windows 开发机 system32 WSL bash 损坏且
        // CreateProcess 搜索序 system32 恒优先于 PATH——PATH 前置无效）：
        // bash 可执行文件路径 env 可配，默认 "bash" 行为零变化（Linux 真机
        // 不受影响；测试经 HEARTH_BASH_BIN 指向 Git Bash）。
        let bash_bin = ctx
            .env
            .get("HEARTH_BASH_BIN")
            .cloned()
            .unwrap_or_else(|| "bash".to_string());
        // S15: 会话 cwd 持久化——包装命令（cd 会话 cwd → 执行 → 尾部回写标记）。
        let wrapped = if persist_enabled() {
            wrap_with_cwd(cmd, &ctx.cwd, get_session_cwd().as_deref())
        } else {
            cmd.to_string()
        };
        let output = match self
            .sandbox
            .spawn(
                &bash_bin,
                &["-c", &wrapped],
                &ctx.cwd,
                &env_refs,
                Duration::from_secs(timeout_secs),
            )
            .await
        {
            Ok(o) => o,
            Err(e) if e.to_string().contains("timed out") => {
                let elapsed = t0.elapsed().as_secs();
                let formatted = s1_timeout_payload(elapsed, timeout_secs, "", "");
                return Err(anyhow::Error::new(tool_runtime::BashExitError {
                    exit_code: -1,
                    timed_out: true,
                    formatted,
                }));
            }
            Err(e) => return Err(e),
        };
        let elapsed = t0.elapsed().as_secs();

        // W4/RC20: 非零退出码/超时 → Err（结构化 error）——scheduler 据此置
        // ToolResult.is_error=true，CLI/渲染层消费结构化字段，不再猜字符串。
        let timed_out = output.timed_out;
        let exit_code = output.exit_code;
        if timed_out {
            // hearth-slim S1: 超时结构化 JSON——{"status":"timeout","elapsed":X,
            // "partial_output":...}（截断后首尾保留）。C-fix-status 挂死 5.5h 的
            // 直接治理：无 deadline 长命令默认 120s 回收，env 可配。
            let formatted =
                s1_timeout_payload(elapsed, timeout_secs, &output.stdout, &output.stderr);
            return Err(anyhow::Error::new(tool_runtime::BashExitError {
                exit_code,
                timed_out: true,
                formatted,
            }));
        }
        // S15: 剥离 cwd 标记 → 回写会话 cwd。**输出零污染**：标记行对 LLM 不可见，
        // 不追加任何投影行（避免破坏输出相等断言/下游解析；cwd 可见性由模型 `pwd` 自查）。
        let base_formatted = format_output(output);
        let (new_cwd, formatted) = if persist_enabled() {
            extract_cwd_marker(&base_formatted)
        } else {
            (None, base_formatted)
        };
        if let Some(p) = &new_cwd {
            set_session_cwd(p.clone());
        }
        // S1: 64KB 工具层截断（sandbox 零触碰——在本层收口防膨胀）。
        let formatted = truncate_output(&formatted, S1_MAX_OUTPUT_BYTES);
        // S2 审计投影：网络命令 → 结果尾部追加 [net] 行（钉死格式，不阻断）。
        let formatted = append_net_audit(cmd, formatted);
        if exit_code != 0 {
            // P2-LR Node 02（RC46 接线）：类型化退出状态（非文本）——scheduler
            // downcast 投影 error_kind（exit >0 / signal / tool timeout 可区分）。
            return Err(anyhow::Error::new(tool_runtime::BashExitError {
                exit_code,
                timed_out: false,
                formatted,
            }));
        }
        Ok(formatted)
    }
}

/// S2: 审计投影行——格式钉死（见 net_audit_hosts 注释），Ok/Err 两路都带。
fn append_net_audit(cmd: &str, formatted: String) -> String {
    match net_audit_hosts(cmd) {
        None => formatted,
        Some(hosts) if hosts.is_empty() => {
            format!("{formatted}\n[net] egress audit: target unresolved")
        }
        Some(hosts) => {
            format!("{formatted}\n[net] egress audit: {}", hosts.join(" "))
        }
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

    /// K-3（契约手术，2026-09-11）：bash 参数形态契约矩阵——
    /// LLM 层 JSON 降级的三种形态下都必须能取到 `cmd`（不得报 "missing 'cmd'"）。
    /// 红样本 = EMBER M1 实测病灶形态（raw String 降级致全部 bash 调用失败）。
    /// 断言不依赖真 shell（Windows/无 bash 环境同样可判"参数提取是否成功"）。
    #[tokio::test]
    async fn test_k3_cmd_arg_three_shapes() {
        use tool_runtime::Tool;
        let tool = BashTool::new();
        let ctx = ToolContext::default();
        let shapes: [(&str, serde_json::Value); 3] = [
            ("①Object", serde_json::json!({"cmd": "echo k3a"})),
            (
                "②raw-String",
                serde_json::Value::String(r#"{"cmd": "echo k3b"}"#.into()),
            ),
            (
                "③truncated-prefix",
                serde_json::Value::String(r#"{"cmd": "echo k3c", "extra": "trunc"#.into()),
            ),
        ];
        for (label, v) in shapes {
            let r = tool.execute(v, &ctx).await;
            if let Err(e) = &r {
                let msg = format!("{e}");
                assert!(
                    !msg.contains("missing 'cmd'"),
                    "K-3 {label}: cmd 必须被取到（实际错误: {msg}）"
                );
            }
        }
    }

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

    // （旧测试 test_egress_empty_allowlist_denies_all 已随 S2 语义反转删除——
    // 空白名单=默认放开 由 test_s2_default_open_empty_allowlist 接管。）

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
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("\"status\":\"timeout\""),
            "S1 结构化超时格式: {msg}"
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

    // ── hearth-slim S1（工具超时，先红后绿）──

    /// S1 支撑：测试用 bash 解析——Windows 开发机 system32 WSL bash 损坏
    /// （Bash/Service/0x8007072c），Git Bash 可用则指向之；Linux 真机不受影响。
    fn test_bash_bin() -> String {
        let candidate = "C:\\Program Files\\Git\\bin\\bash.exe";
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
        "bash".to_string()
    }

    /// S1: 超时解析纯函数——effective(deadline) > args > env > 默认 120，cap 600。
    #[test]
    fn test_s1_resolve_timeout_priority() {
        use std::collections::HashMap;
        use std::time::Duration;
        let empty: HashMap<String, String> = HashMap::new();
        let args = serde_json::json!({});
        // 默认 120
        assert_eq!(resolve_timeout_secs(&args, None, &empty), 120);
        // env 覆盖默认
        let mut env5: HashMap<String, String> = HashMap::new();
        env5.insert("HEARTH_TOOL_TIMEOUT_SECS".into(), "5".into());
        assert_eq!(resolve_timeout_secs(&args, None, &env5), 5);
        // args 覆盖 env
        let args300 = serde_json::json!({"timeout_secs": 300});
        assert_eq!(resolve_timeout_secs(&args300, None, &env5), 300);
        // effective（任务剩余期）最高，且 cap 600
        assert_eq!(
            resolve_timeout_secs(&args300, Some(Duration::from_secs(30)), &env5),
            30
        );
        let args_big = serde_json::json!({"timeout_secs": 9999});
        assert_eq!(resolve_timeout_secs(&args_big, None, &empty), 600);
    }

    /// S1: 超时返回结构化 JSON（status/elapsed/partial_output）——env 2s 回收 sleep 10。
    #[tokio::test]
    async fn test_s1_timeout_structured_json() {
        let tool = BashTool::new();
        let mut ctx = ToolContext::default();
        ctx.env
            .insert("HEARTH_TOOL_TIMEOUT_SECS".to_string(), "2".to_string());
        ctx.env
            .insert("HEARTH_BASH_BIN".to_string(), test_bash_bin());
        let t0 = std::time::Instant::now();
        let result = tool
            .execute(serde_json::json!({"cmd": "sleep 10"}), &ctx)
            .await;
        let elapsed = t0.elapsed().as_secs();
        assert!(result.is_err(), "超时必须 Err");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("\"status\":\"timeout\""),
            "须含 status=timeout: {msg}"
        );
        assert!(msg.contains("elapsed"), "须含 elapsed: {msg}");
        assert!(msg.contains("partial_output"), "须含 partial_output: {msg}");
        assert!(
            elapsed <= 8,
            "env 2s 必须在 ~2s 回收（实测 {elapsed}s）——5.5h 挂死复发即红"
        );
    }

    /// S1: stdout >64KB 截断并标记（防 45 万 tokens 式膨胀复发）。
    #[tokio::test]
    async fn test_s1_stdout_truncated_64kb() {
        let tool = BashTool::new();
        let mut ctx = ToolContext::default();
        ctx.env
            .insert("HEARTH_BASH_BIN".to_string(), test_bash_bin());
        let result = tool
            .execute(serde_json::json!({"cmd": "seq 1 40000"}), &ctx)
            .await
            .unwrap();
        assert!(result.contains("[truncated]"), "超 64KB 输出必须带截断标记");
        assert!(
            result.len() < 200 * 1024,
            "截断后输出不得再是巨量: {} bytes",
            result.len()
        );
        // 首尾保留（头部 1 与尾部 40000 都在）
        assert!(result.contains("1"), "头部内容保留");
        assert!(result.contains("40000"), "尾部内容保留");
    }

    // ── hearth-slim S2（出网默认放开，先红后绿）──
    // 审计投影格式（顶层附加要求：落刀前钉死，防逐卡漂移）：
    //   触发特征 = 命令含 URL 或已知网络命令（check_egress 同一判定）；
    //   追加行样式 = `[net] egress audit: <host1> <host2> …`（单行、追加在
    //   结果尾部、不阻断不解析）；目标不可解析时 = `[net] egress audit: target
    //   unresolved`。风险披露：提示注入/恶意命令风险已知悉接受（用户拍板
    //   2026-09-09），审计投影为唯一缓解层。

    /// S2: 默认放开——白名单空/未设 = 全放（语义反转，先红：旧实现全拒）。
    #[test]
    fn test_s2_default_open_empty_allowlist() {
        let env = env_with_allowlist("");
        assert!(
            check_egress("curl https://example.com/", &env).is_ok(),
            "S2 语义反转：空白名单 = 默认放开"
        );
        let env2 = std::collections::HashMap::new();
        assert!(
            check_egress("curl https://example.com/", &env2).is_ok(),
            "未设 allowlist = 默认放开"
        );
    }

    /// S2: 显式 allowlist 仍然收紧（保留 HEARTH_EGRESS_ALLOWLIST 收紧能力）。
    #[test]
    fn test_s2_allowlist_still_tightens() {
        let env = env_with_allowlist("example.com");
        let e = check_egress("curl https://evil.test/", &env).unwrap_err();
        assert!(e.contains("evil.test"), "收紧模式仍按白名单拒绝: {e}");
    }

    /// S2: 审计投影——网络命令执行成功后结果尾部带 [net] 行（含主机清单）。
    #[tokio::test]
    async fn test_s2_audit_projection_appended() {
        let tool = BashTool::new();
        let mut ctx = ToolContext::default();
        ctx.env
            .insert("HEARTH_BASH_BIN".to_string(), test_bash_bin());
        let result = tool
            .execute(
                serde_json::json!({"cmd": "echo curl-probe && curl -s -m 10 https://example.com -o /dev/null; echo done"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(
            result.contains("[net] egress audit: example.com"),
            "结果尾部必须带 [net] 审计行（钉死格式）: {result}"
        );
        assert!(result.contains("done"), "原输出保留: {result}");
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

// ── S15（手术包二）：会话 cwd 持久化——纯函数单测（无 env/无进程态，无竞态） ──
#[cfg(test)]
mod s15_tests {
    use super::*;

    #[test]
    fn test_wrap_with_cwd_uses_session_over_base() {
        let base = std::path::Path::new("/base");
        let sess = std::path::Path::new("/deep/dir");
        // 无会话 cwd → 用 base
        let w1 = wrap_with_cwd("ls", base, None);
        assert!(w1.contains("cd '/base'"), "无会话 cwd 用 base：{w1}");
        assert!(w1.contains("ls"), "用户命令保留");
        assert!(w1.contains("exit $__hearth_rc"), "退出码保持");
        assert!(w1.contains(CWD_MARKER), "尾部回写标记在位");
        // 有会话 cwd → 会话优先（base 是首次起点，会话是当前）
        let w2 = wrap_with_cwd("pwd", base, Some(sess));
        assert!(w2.contains("cd '/deep/dir'"), "会话 cwd 优先：{w2}");
    }

    #[test]
    fn test_extract_cwd_marker_strips_and_parses() {
        // 正常：末行标记 → 解析 + 剥离（LLM 不见标记行）
        let out = "hello\n__HEARTH_CWD__={/home/u/proj}\n";
        let (cwd, clean) = extract_cwd_marker(out);
        assert_eq!(cwd, Some(std::path::PathBuf::from("/home/u/proj")));
        assert!(!clean.contains(CWD_MARKER), "标记必须剥离：{clean}");
        assert!(clean.contains("hello"), "正文保留");
        // 取最后一处（命令自身回显的早期标记行**保留**，防伪造；仅尾部真实标记剥离）
        let out2 = "echo __HEARTH_CWD__={/early}\n__HEARTH_CWD__={/late}";
        let (cwd2, clean2) = extract_cwd_marker(out2);
        assert_eq!(cwd2, Some(std::path::PathBuf::from("/late")), "取最后标记");
        assert!(clean2.contains("{/early}"), "早出现的回显保留（非尾部标记）");
        assert!(!clean2.contains("/late}"), "尾部标记已剥离：{clean2}");
        // 无标记 → (None, 原样)
        let (cwd3, clean3) = extract_cwd_marker("plain output");
        assert!(cwd3.is_none());
        assert_eq!(clean3, "plain output");
    }

    #[test]
    fn test_extract_cwd_marker_rejects_empty() {
        let (cwd, clean) = extract_cwd_marker("x\n__HEARTH_CWD__={}\n");
        assert!(cwd.is_none(), "空路径不采纳");
        assert!(clean.contains("x"));
    }

    #[test]
    fn test_session_cwd_roundtrip_and_toggle() {
        // 直读写回（测试隔离：本测试只碰自己的值，不依赖 env）
        let p = std::path::PathBuf::from("/tmp/s15-roundtrip");
        set_session_cwd(p.clone());
        assert_eq!(get_session_cwd(), Some(p));
        // 开关确定性（不设 env 时默认开）
        if std::env::var("HEARTH_BASH_NO_PERSIST").is_err() {
            assert!(persist_enabled(), "默认开启持久化");
        }
    }
}
