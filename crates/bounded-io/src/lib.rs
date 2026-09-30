//! 有界读入 / 有界排空 / 进程树回收 —— 一组**跨 crate 共用的安全原语**。
//!
//! # 这个 crate 为什么存在（D-33 收敛）
//!
//! 2026-10-01 的连续几张卡（P0-06 / P0-07 / P0-08 / P0-09 / P0-12）反复修同一个
//! 缺陷族——**"读入无上限"与"进程不收尸"**，结果在**四个 crate**里各留了一份
//! 实现（`sandbox` / `tools-builtin` / `agent-runtime` / `agent-core`）。
//!
//! 四份实现的关键语义**必须完全一致**，否则修一处漏三处：
//!
//! 1. **必须"读到 EOF"**——`Read::take(cap)` 会在达到上限后停止读取，
//!    子进程随即**写满管道并永久阻塞**（把 OOM 换成死锁）。正确做法是**继续排空、
//!    只保留前 `cap` 字节**；
//! 2. **截断必须留痕**——调用方要知道自己看的不是全貌（"不静默"红线）；
//! 3. **文本截断要退到合法 UTF-8 边界**——否则会在字符串里插入半个字符；
//! 4. **杀进程要杀到"树"**——`Child::kill` 只到直接子进程；`cmd /C start …`
//!    或 `cargo check` 派生的 rustc 会成孤儿（Windows 用系统自带 `taskkill /T`）。
//!
//! 顶层裁决：**收敛**（原为"sandbox / tools-builtin::read / agent-core 三处重复"）。
//! 收敛后这四条不变式只有**一处定义**。

use std::io::Read;
use std::path::Path;

/// 单条流 / 单个文件默认保留的字节上限。
///
/// 8 MiB 的取法：远大于任何正常场景（命令输出、源码文件、产物预览都在数百 KiB
/// 量级），同时把"失控输入"从"OOM 整个进程"退化为"截断 + 明确留痕"。
pub const MAX_CAPTURED_BYTES: usize = 8 * 1024 * 1024;

/// 排空一个**同步**读端（阻塞读线程用），最多保留 `cap` 字节。
///
/// 返回 `(保留数据, 是否发生截断)`。读错误按"拿到多少算多少"处理——
/// 不因排空失败丢掉整段输出。
pub fn drain_capped_std<R: Read>(mut r: R, cap: usize) -> (Vec<u8>, bool) {
    let mut kept = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    let mut truncated = false;
    loop {
        match r.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let room = cap.saturating_sub(kept.len());
                if n > room {
                    truncated = true;
                }
                kept.extend_from_slice(&chunk[..room.min(n)]);
            }
            Err(_) => break,
        }
    }
    (kept, truncated)
}

/// 排空一个**异步**读端，语义同 [`drain_capped_std`]。
pub async fn drain_capped_async<R: tokio::io::AsyncRead + Unpin>(
    mut r: R,
    cap: usize,
) -> (Vec<u8>, bool) {
    use tokio::io::AsyncReadExt;
    let mut kept = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    let mut truncated = false;
    loop {
        match r.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                let room = cap.saturating_sub(kept.len());
                if n > room {
                    truncated = true;
                }
                kept.extend_from_slice(&chunk[..room.min(n)]);
            }
            Err(_) => break,
        }
    }
    (kept, truncated)
}

/// 有界读取文本文件。返回 `(内容, 是否被截断)`。
///
/// - 最多读 `cap` 字节（多读 1 字节用于判断"后面还有没有"）；
/// - **截断时**退到最后一个合法 UTF-8 边界（截断点可能切断多字节字符，
///   但原文件本身是合法 UTF-8，只是我们主动只读了一段）；
/// - **未截断时**保持严格语义：非 UTF-8 一律报错（不静默 lossy）。
pub async fn read_file_text_capped(path: &Path, cap: u64) -> std::io::Result<(String, bool)> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut raw = Vec::new();
    (&mut file).take(cap + 1).read_to_end(&mut raw).await?;
    let truncated = raw.len() as u64 > cap;
    if truncated {
        raw.truncate(cap as usize);
    }
    let text = if truncated {
        match String::from_utf8(raw) {
            Ok(s) => s,
            Err(e) => {
                let valid = e.utf8_error().valid_up_to();
                let bytes = e.into_bytes();
                String::from_utf8(bytes[..valid].to_vec())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            }
        }
    } else {
        String::from_utf8(raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
    };
    Ok((text, truncated))
}

/// 杀掉以 `pid` 为根的**整棵进程树**；返回 `true` = 已执行树杀。
///
/// 为什么不能只靠 `Child::kill` / `kill_on_drop`：它们只终止**直接子进程**。
/// Windows 上 `cmd /C start …`、`bash -c "… &"` 派生的孙进程不在其列 ——
/// 超时后会变成孤儿继续跑（占 CPU / 端口 / 文件锁），而父进程已经报"超时"走人。
///
/// - Windows：`taskkill /T /F /PID`（系统自带，**不引入新依赖**）；
/// - 其他平台：无等价系统工具 → 返回 `false`，调用方退化为直接 kill。
///   （Linux **生产**路径不经过这里：`sandbox::LinuxSandbox` 用 setsid +
///   `kill(-pgid)` 组杀。）
pub fn kill_process_tree(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(windows)]
    {
        match std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .output()
        {
            Ok(o) if o.status.success() => true,
            Ok(o) => {
                tracing::warn!(
                    "kill_process_tree: taskkill 失败 (pid {pid}): {}",
                    String::from_utf8_lossy(&o.stderr).trim()
                );
                false
            }
            Err(e) => {
                tracing::warn!("kill_process_tree: 无法执行 taskkill (pid {pid}): {e}");
                false
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        false
    }
}

// ── D-55/D-58：HTTP 响应体有界读取（reqwest）──
//
// `resp.text()` 会把**整份响应**读进内存（reqwest 无内建上限）——超大/恶意响应
// 可致 OOM。本原语是本仓**唯一**的 HTTP body 有界读取实现（D-33 收敛）：
// 由 `tools-builtin` 再导出，并被 `codex-cli` 与各 `llm-*` provider 复用。
//
// 门控在 `reqwest` feature 之后——不需要 HTTP 的消费方（如 `memory`）默认不拉入
// reqwest/anyhow。

/// D-55（2026-10-01）：**有界读取 HTTP 响应体**的字节上限。
///
/// 1 MiB ≈ 输出上限的 130 倍，足以容纳正常文档页的 HTML 标记，同时把"失控响应"
/// 从 OOM 退化为"截断 + 留痕"。
#[cfg(feature = "reqwest")]
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// D-55：有界读取 reqwest 响应体（**网络侧**的"无界读入"落点，与文件/管道落点同族）。
///
/// - 先用 `Content-Length`（若有）**提前拒绝**超大响应——不必下载就知道超限；
/// - 否则用 `Response::chunk()` 流式累加，**最多保留 `cap` 字节**；超出即停读并丢弃余量
///   （HTTP 与管道不同：丢弃后续字节不会让写端死锁，只是关闭连接）；
/// - 截断处退到最后一个合法 UTF-8 边界。本仓 reqwest **未启用 `charset` feature**，
///   `.text()` 本就是 UTF-8 lossy → `from_utf8_lossy` 与旧行为等价；
/// - 截断**留痕**（不静默）。
#[cfg(feature = "reqwest")]
pub async fn read_body_capped(
    mut resp: reqwest::Response,
    cap: usize,
) -> anyhow::Result<(String, bool)> {
    if let Some(len) = resp.content_length() {
        if len > cap as u64 {
            anyhow::bail!(
                "响应体过大：Content-Length {len} 字节 > 上限 {cap} 字节——已拒绝（避免整份读入内存）"
            );
        }
    }

    let mut buf: Vec<u8> = Vec::with_capacity(cap.min(64 * 1024));
    let mut truncated = false;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| anyhow::anyhow!("读 body 失败: {e}"))?
    {
        let room = cap.saturating_sub(buf.len());
        if chunk.len() > room {
            buf.extend_from_slice(&chunk[..room]);
            truncated = true;
            break;
        }
        buf.extend_from_slice(&chunk);
    }

    let text = if truncated {
        match String::from_utf8(buf) {
            Ok(s) => s,
            Err(e) => {
                let valid = e.utf8_error().valid_up_to();
                let bytes = e.into_bytes();
                String::from_utf8_lossy(&bytes[..valid]).into_owned()
            }
        }
    } else {
        String::from_utf8_lossy(&buf).into_owned()
    };

    if truncated {
        tracing::warn!(cap, "HTTP 响应体超过上限，已截断（丢弃余量）");
    }
    Ok((text, truncated))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 先红后绿（红侧）：修复前各处用的是 `Read::read_to_end`——**无上限**，
    /// 10 GiB 输出就吃 10 GiB 内存。绿侧要求：保留量被 `cap` 钳住。
    #[test]
    fn test_drain_capped_std_bounds_kept_bytes_and_drains_to_eof() {
        let data = vec![b'a'; 100 * 1024];
        let mut cursor = std::io::Cursor::new(data.clone());
        let (kept, truncated) = drain_capped_std(&mut cursor, 4096);
        assert_eq!(kept.len(), 4096, "保留量必须被上限钳住（修复前等于全长）");
        assert!(truncated, "超限必须报告截断（不静默）");
        assert_eq!(
            cursor.position(),
            data.len() as u64,
            "必须**继续排空到 EOF**——若图省事用 `take(cap)` 提前停读，\
             子进程会因管道写满而永久阻塞（把 OOM 换成死锁）"
        );
    }

    #[test]
    fn test_drain_capped_std_exact_fit_is_not_truncated() {
        let data = vec![b'x'; 4096];
        let (kept, truncated) = drain_capped_std(&data[..], 4096);
        assert_eq!(kept.len(), 4096);
        assert!(!truncated, "恰好等于上限不算截断");
    }

    #[test]
    fn test_drain_capped_std_empty_and_small_input() {
        let (kept, truncated) = drain_capped_std(&b""[..], 4096);
        assert!(kept.is_empty());
        assert!(!truncated);

        let (kept, truncated) = drain_capped_std(&b"hi"[..], 4096);
        assert_eq!(kept, b"hi");
        assert!(!truncated);
    }

    /// 异步版语义必须与同步版一致（NoopSandbox / 自检命令走这条）。
    #[tokio::test]
    async fn test_drain_capped_async_bounds_kept_bytes() {
        use tokio::io::AsyncWriteExt;
        let (mut w, r) = tokio::io::duplex(64 * 1024);
        let payload = vec![b'z'; 100 * 1024];
        let writer = tokio::spawn(async move {
            // 读者并发消费，duplex 有界也不会死锁；写完后关写端让读者命中 EOF。
            let _ = w.write_all(&payload).await;
            drop(w);
        });
        let (kept, truncated) = drain_capped_async(r, 8192).await;
        writer.await.unwrap();
        assert_eq!(kept.len(), 8192, "异步版同样必须有界");
        assert!(truncated);
    }

    /// 先红后绿（红侧）：修复前各调用点用 `read_to_string` 把**整份文件**读进内存。
    #[tokio::test]
    async fn test_read_file_text_capped_bounds_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.txt");
        std::fs::write(&p, "C".repeat(4096 + 100)).unwrap();

        let (text, truncated) = read_file_text_capped(&p, 4096).await.unwrap();
        assert_eq!(
            text.len(),
            4096,
            "保留量必须被 cap 钳住（修复前等于整份文件长度）"
        );
        assert!(truncated, "超限必须报告截断（不静默）");

        // 边界：恰好等于上限 → 不算截断。
        let exact = dir.path().join("exact.txt");
        std::fs::write(&exact, "D".repeat(4096)).unwrap();
        let (text, truncated) = read_file_text_capped(&exact, 4096).await.unwrap();
        assert_eq!(text.len(), 4096);
        assert!(!truncated);
    }

    /// 边界：截断点切断多字节字符时，须退到合法 UTF-8 边界
    /// （既不引入替换符，也不因此报错）。
    #[tokio::test]
    async fn test_read_file_text_capped_utf8_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cjk.txt");
        std::fs::write(&p, "中文文件").unwrap(); // 每字 3 字节，共 12 字节
        let (text, truncated) = read_file_text_capped(&p, 4).await.unwrap();
        assert!(truncated);
        assert_eq!(text, "中", "cap=4 切在第二个字中间，必须退到合法边界");
    }

    /// 边界：**未**截断时保持严格语义——非 UTF-8 一律报错（不静默 lossy）。
    #[tokio::test]
    async fn test_read_file_text_capped_rejects_non_utf8_when_not_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bin.dat");
        std::fs::write(&p, [0xff_u8, 0xfe, 0x00, 0x01]).unwrap();
        assert!(read_file_text_capped(&p, 4096).await.is_err());
    }

    /// `kill_process_tree` 本身不做单测：它只能在**真实有子进程**时有意义，
    /// 覆盖由两个真实调用方端到端完成——
    /// `sandbox::p0_07_tests::test_p0_07_noop_timeout_kills_grandchildren`
    /// 与 `agent_core::loop::tests::test_p0_09_run_check_cmd_timeout_reaps_child`
    /// （两者都会断言目标进程确实消失）。
    #[test]
    fn test_kill_process_tree_zero_pid_is_noop() {
        assert!(
            !kill_process_tree(0),
            "pid=0 必须直接返回 false（不误杀进程组）"
        );
    }

    // ── D-55：HTTP 响应体有界读取（reqwest feature）──

    /// 起一个一次性本地 HTTP 服务（回固定 body），返回 URL。
    #[cfg(feature = "reqwest")]
    async fn serve_once(content_length: Option<usize>, body_len: usize) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut req = [0u8; 1024];
                let _ = sock.read(&mut req).await;
                let cl = match content_length {
                    Some(n) => format!("Content-Length: {n}\r\n"),
                    None => String::new(),
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n{cl}Connection: close\r\n\r\n"
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let chunk = vec![b'a'; 64 * 1024];
                let mut left = body_len;
                while left > 0 {
                    let n = left.min(chunk.len());
                    if sock.write_all(&chunk[..n]).await.is_err() {
                        break;
                    }
                    left -= n;
                }
                let _ = sock.flush().await;
            }
        });
        format!("http://{addr}/")
    }

    /// 先红后绿（D-55）：`Content-Length` 超限必须**提前拒绝**（不下载整份）。
    /// 修复前走 `resp.text()` → 整份入内存（本测会拿到 Ok 而非 Err）。
    #[cfg(feature = "reqwest")]
    #[tokio::test]
    async fn test_d55_body_capped_rejects_oversized_content_length() {
        let url = serve_once(Some(5_000_000), 5_000_000).await;
        let resp = reqwest::get(&url).await.unwrap();
        let err = read_body_capped(resp, 1024)
            .await
            .expect_err("Content-Length 超限必须被提前拒绝（不整份读入）");
        assert!(err.to_string().contains("响应体过大"), "got: {err}");
    }

    /// 正常小响应原样返回、不标截断（保证修复不误伤）。
    #[cfg(feature = "reqwest")]
    #[tokio::test]
    async fn test_d55_body_capped_small_body_ok() {
        let url = serve_once(Some(5), 5).await;
        let resp = reqwest::get(&url).await.unwrap();
        let (body, truncated) = read_body_capped(resp, 1024).await.unwrap();
        assert_eq!(body, "aaaaa");
        assert!(!truncated, "未超限不得标截断");
    }
}
