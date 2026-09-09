//! LSP bridge: async LSP client for diagnostics + completion.
//!
//! Provides LspBridge trait + NoopLspBridge (graceful degradation) +
//! RustAnalyzerBridge (real rust-analyzer subprocess via JSON-RPC).
//! LSP unavailable → empty diagnostics, no panic (analogous to NoopSandbox).

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

/// A diagnostic message from the language server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// The LspBridge trait — async LSP client abstraction.
#[async_trait]
pub trait LspBridge: Send + Sync {
    /// Fetch diagnostics for a given file.
    async fn diagnostics(&self, file: &Path) -> Result<Vec<Diagnostic>>;

    /// Whether a real LSP server is connected.
    fn is_connected(&self) -> bool;

    /// Human-readable backend name.
    fn backend_name(&self) -> &str;
}

// ── NoopLspBridge (graceful degradation) ──

pub struct NoopLspBridge;

impl NoopLspBridge {
    pub fn new() -> Self {
        tracing::warn!(
            "NoopLspBridge: no LSP server — diagnostics will be empty. Only for development."
        );
        Self
    }
}

#[async_trait]
impl LspBridge for NoopLspBridge {
    async fn diagnostics(&self, _file: &Path) -> Result<Vec<Diagnostic>> {
        Ok(Vec::new())
    }

    fn is_connected(&self) -> bool {
        false
    }

    fn backend_name(&self) -> &str {
        "noop"
    }
}

impl Default for NoopLspBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ── RustAnalyzerBridge (P3: real LSP client) ──

pub struct RustAnalyzerBridge {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    stdout: Mutex<Option<BufReader<ChildStdout>>>,
    connected: AtomicBool,
    init_done: AtomicBool,
    /// H3: handle for background init task (stored so drop doesn't cancel).
    #[allow(dead_code)]
    init_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl RustAnalyzerBridge {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            stdin: Mutex::new(None),
            stdout: Mutex::new(None),
            connected: AtomicBool::new(false),
            init_done: AtomicBool::new(false),
            init_handle: Mutex::new(None),
        }
    }

    /// Spawn rust-analyzer subprocess (called lazily on first diagnostics() call).
    async fn ensure_started(&self) -> Result<()> {
        // H3: only proceed if both process and init are complete
        if self.connected.load(Ordering::Relaxed) && self.init_done.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut child_guard = self.child.lock().await;
        if child_guard.is_none() {
            tracing::info!("RustAnalyzerBridge: spawning rust-analyzer");
            let mut child = Command::new("rust-analyzer")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .context("failed to spawn rust-analyzer — ensure it is installed (rustup component add rust-analyzer)")?;

            let stdin = child
                .stdin
                .take()
                .context("rust-analyzer stdin unavailable")?;
            let stdout = child
                .stdout
                .take()
                .context("rust-analyzer stdout unavailable")?;

            *self.stdin.lock().await = Some(stdin);
            *self.stdout.lock().await = Some(BufReader::new(stdout));

            self.connected.store(true, Ordering::Relaxed);
            *child_guard = Some(child);
        }
        drop(child_guard);

        // H3: retry init on each call until success (60s timeout, up from 10s)
        if !self.init_done.load(Ordering::Relaxed) {
            self.initialize().await?;
        }
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        if self.init_done.load(Ordering::Relaxed) {
            return Ok(());
        }

        let root_uri = std::env::current_dir()
            .ok()
            .map(|d| format!("file://{}", d.display()))
            .unwrap_or_else(|| "file:///home/wutao/codex".to_string());

        let init_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {}
            }
        });
        self.send_json(&init_req).await?;

        // Wait for initialize response (H3: 60s timeout for first full workspace scan)
        let _resp = timeout(Duration::from_secs(60), self.read_message()).await??;
        tracing::debug!("RustAnalyzerBridge: initialize response received");

        // Send initialized notification
        let init_done_notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        self.send_json(&init_done_notif).await?;

        self.init_done.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Send a JSON-RPC message with Content-Length framing.
    async fn send_json(&self, value: &Value) -> Result<()> {
        let body = serde_json::to_string(value)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let msg = format!("{}{}", header, body);

        let mut stdin = self.stdin.lock().await;
        let stdin = stdin
            .as_mut()
            .context("rust-analyzer stdin not available")?;
        stdin.write_all(msg.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Read a JSON-RPC message with Content-Length framing.
    async fn read_message(&self) -> Result<Value> {
        let mut stdout = self.stdout.lock().await;
        let stdout = stdout
            .as_mut()
            .context("rust-analyzer stdout not available")?;

        let mut header_line = String::new();
        stdout.read_line(&mut header_line).await?;
        let header_line = header_line.trim().to_lowercase();

        if !header_line.starts_with("content-length:") {
            anyhow::bail!("expected Content-Length header, got: {}", header_line);
        }

        let len_str = header_line.strip_prefix("content-length:").unwrap().trim();
        let content_length: usize = len_str.parse().context("invalid Content-Length")?;

        // Consume blank line separator
        let mut blank = String::new();
        stdout.read_line(&mut blank).await?;

        // Read the JSON body
        let mut body = vec![0u8; content_length];
        stdout
            .read_exact(&mut body)
            .await
            .context("failed to read JSON-RPC body")?;

        let value: Value = serde_json::from_slice(&body)?;
        Ok(value)
    }

    /// Read a JSON-RPC message with Content-Length framing.
    fn parse_diagnostics(notification: &Value, file: &Path) -> Vec<Diagnostic> {
        let params = match notification.get("params") {
            Some(p) => p,
            None => return vec![],
        };
        let uri = match params.get("uri").and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return vec![],
        };
        // Only return diagnostics for the requested file
        if !uri.ends_with(file.to_string_lossy().as_ref()) {
            return vec![];
        }
        let diags = match params.get("diagnostics").and_then(|d| d.as_array()) {
            Some(d) => d,
            None => return vec![],
        };
        diags
            .iter()
            .map(|d| {
                let range = d.get("range").and_then(|r| r.get("start"));
                let line = range
                    .and_then(|s| s.get("line").and_then(|l| l.as_u64()))
                    .unwrap_or(0) as usize;
                let column = range
                    .and_then(|s| s.get("character").and_then(|c| c.as_u64()))
                    .unwrap_or(0) as usize;
                Diagnostic {
                    file: file.to_path_buf(),
                    line,
                    column,
                    end_line: line,
                    end_column: column + 1,
                    severity: DiagnosticSeverity::Warning,
                    message: d
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_string(),
                    source: "rust-analyzer".to_string(),
                }
            })
            .collect()
    }
}

#[async_trait]
impl LspBridge for RustAnalyzerBridge {
    async fn diagnostics(&self, file: &Path) -> Result<Vec<Diagnostic>> {
        // Lazy init: spawn on first call
        if self.ensure_started().await.is_err() {
            return Ok(Vec::new()); // graceful degradation
        }

        // Send textDocument/didOpen
        let uri = format!("file://{}", file.display());
        let did_open = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": ""
                }
            }
        });
        let _ = self.send_json(&did_open).await;

        // Wait for publishDiagnostics (or timeout)
        let result: std::result::Result<
            anyhow::Result<Vec<Diagnostic>>,
            tokio::time::error::Elapsed,
        > = timeout(Duration::from_secs(5), async {
            loop {
                let msg = self.read_message().await?;
                if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                    if method == "textDocument/publishDiagnostics" {
                        return Ok(Self::parse_diagnostics(&msg, file));
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Ok(diags)) => Ok(diags),
            _ => Ok(Vec::new()), // timeout or error → empty
        }
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    fn backend_name(&self) -> &str {
        "rust-analyzer"
    }
}

impl Default for RustAnalyzerBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_bridge_returns_empty() {
        let bridge = NoopLspBridge::new();
        assert!(!bridge.is_connected());
        assert_eq!(bridge.backend_name(), "noop");

        let diags = bridge.diagnostics(&PathBuf::from("test.rs")).await.unwrap();
        assert!(
            diags.is_empty(),
            "noop bridge should return empty diagnostics"
        );
    }

    #[tokio::test]
    async fn test_noop_bridge_does_not_panic() {
        // A3: graceful degradation — no panic on missing server
        let bridge = NoopLspBridge::new();
        let result = bridge
            .diagnostics(&PathBuf::from("/nonexistent/file.rs"))
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
