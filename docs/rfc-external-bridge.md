# RFC: External Bridge — 多窗口外部智能接入层

| 字段 | 值 |
|------|-----|
| **RFC 编号** | RFC-001 |
| **标题** | External Bridge — 多窗口外部智能接入层 |
| **作者** | 元宝（外部顾问） |
| **状态** | Draft（待评估） |
| **目标版本** | v16 |
| **依赖** | codex-rust v15.0（P1/P2/P3 止血完成） |
| **参考** | MCP 协议规范、codex-mcp-rs、AGENTS.md V4.0 |

---

## 0. 摘要（TL;DR）

本 RFC 为 codex-rust 设计一个名为 **External Bridge（外部桥）** 的新 crate（`crates/external-bridge`），它允许外部 AI 个体（如元宝、Kimi、Claude 等）通过标准化协议接入启元的身体，获得一个**独立窗口**，在该窗口内调用启元身体的工具、与其他窗口通信，所有行为受启元基因（宪法引擎 + xray + 沙箱）约束。

**核心一句话**：External Bridge 是启元身体的"多租户旅馆"——每个外部 AI 一个房间，共用工具，遵守同一部宪法。

---

## 1. 背景与动机

### 1.1 当前架构的局限

codex-rust v15 的架构是**单 LLM + 单 Agent 实例**的模式：

- `service` 层通过 9 路 LLM provider 路由，但每次只有一个活跃会话
- `agent-core` 的 loop.rs 驱动五阶段状态机，但它是**单线程串行**的
- 没有机制让多个异构 AI 同时接入、协作、共享工具

### 1.2 为什么需要外部桥

| 需求 | 当前能力 | 外部桥解决 |
|------|---------|------------|
| 多个 AI 同时接入 | ❌ 单会话 | ✅ 每 AI 一个独立窗口 |
| 外部 AI 调用启元工具 | ❌ 无接口 | ✅ 标准化工具代理 |
| 窗口间协作通信 | ❌ 无机制 | ✅ 消息总线 |
| 外部 AI 受启元基因约束 | ❌ 无接入点 | ✅ 安全闸门统一拦截 |
| 跨 provider 对比自动化 | ❌ 手动跑 | ✅ 窗口级隔离测试 |

### 1.3 与 MCP 的关系

External Bridge **兼容 MCP 协议**（Model Context Protocol），但不仅限于 MCP：

- MCP Server 模式：外部桥作为 MCP Server 运行，任何 MCP 兼容的 Host（Claude Desktop、Codex CLI、Cursor）可直接连接
- REST + JSON 模式：对于不支持 MCP 的客户端，提供 HTTP API
- 内部 Rust API：对于同为 Rust 生态的客户端，可直接 `use external_bridge::*`

三种模式共享同一套核心引擎（窗口管理 + 消息总线 + 安全闸门）。

---

## 2. 架构设计

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                    外部 AI 个体（异构）                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  元宝     │  │  Kimi K3  │  │  Claude  │  │  其他...  │   │
│  │ (我的窗口)│  │ (他的窗口)│  │ (她的窗口)│  │           │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
└───────┼───────────────┼─────────────┼─────────────┼───────────┘
        │               │             │             │
        └───────────────┴─────────────┴─────────────┘
                            ▼ MCP / REST / gRPC
        ┌──────────────────────────────────────────────────┐
        │          crates/external-bridge（本 RFC 主体）      │
        │  ┌────────────────────────────────────────────┐  │
        │  │          WindowManager（窗口管理）            │  │
        │  │  create / destroy / query / timeout         │  │
        │  └────────────────┬───────────────────────────┘  │
        │  ┌────────────────┼───────────────────────────┐  │
        │  │          MessageBus（消息总线）               │  │
        │  │  point-to-point / broadcast / group         │  │
        │  └────────────────┬───────────────────────────┘  │
        │  ┌────────────────┼───────────────────────────┐  │
        │  │          ToolProxy（工具代理）                 │  │
        │  │  权限检查 → xray → sandbox → tool-runtime  │  │
        │  └────────────────┬───────────────────────────┘  │
        │  ┌────────────────┼───────────────────────────┐  │
        │  │          SecurityGate（安全闸门）              │  │
        │  │  宪法引擎 + xray + landlock + 配额          │  │
        │  └────────────────┬───────────────────────────┘  │
        └───────────────────┼──────────────────────────────┘
                            ▼
        ┌──────────────────────────────────────────────────┐
        │               codex-rust 身体（现有）              │
        │  service │ agent-core │ tool-runtime │ sandbox   │
        │  project-xray │ constitution │ experience │ memory │
        └──────────────────────────────────────────────────┘
```

### 2.2 核心数据结构

```rust
// crates/external-bridge/src/lib.rs

use uuid::Uuid;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};

/// 一个外部 AI 的接入窗口
pub struct Window {
    /// 全局唯一窗口 ID
    pub id: Uuid,
    /// 窗口所有者标识（如 "yuanbao", "kimi-k3", "claude-sonnet-4"）
    pub owner: String,
    /// 窗口创建时间
    pub created_at: std::time::Instant,
    /// 最后活跃时间（用于超时回收）
    pub last_active: std::sync::atomic::AtomicU64,
    /// 该窗口允许调用的工具集合
    pub permissions: HashSet<ToolPermission>,
    /// 独立的沙箱工作目录
    pub sandbox_dir: PathBuf,
    /// 消息接收通道
    pub message_rx: mpsc::Receiver<Message>,
    /// 消息发送通道（由 WindowManager 持有）
    pub message_tx: mpsc::Sender<Message>,
    /// 窗口状态
    pub state: Arc<RwLock<WindowState>>,
    /// 工具调用配额（剩余次数）
    pub tool_quota_remaining: std::sync::atomic::AtomicU32,
    /// 操作审计日志路径
    pub audit_log_path: PathBuf,
}

/// 窗口状态机
pub enum WindowState {
    /// 创建中（尚未完成初始化）
    Initializing,
    /// 活跃（可正常调用工具和收发消息）
    Active,
    /// 挂起（超时未活动，等待恢复）
    Suspended { reason: String },
    /// 销毁中
    Destroying,
    /// 已销毁
    Destroyed,
}

/// 工具权限枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolPermission {
    /// 读取文件
    ReadFile,
    /// 写入文件（在沙箱内）
    WriteFile,
    /// 编辑文件（在沙箱内）
    EditFile,
    /// 运行命令（在沙箱内）
    RunCommand,
    /// 编译检查
    CargoCheck,
    /// 运行测试
    CargoTest,
    /// 代码 lint
    CargoClippy,
    /// 搜索（grep/glob）
    Search,
    /// 发送消息给其他窗口
    SendMessage,
    /// 广播消息
    BroadcastMessage,
    /// 读取经验库
    ReadExperience,
    /// 写入经验库
    WriteExperience,
}

/// 跨窗口消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息唯一 ID
    pub id: Uuid,
    /// 发送方窗口 ID
    pub from: Uuid,
    /// 接收方
    pub to: Recipient,
    /// 消息类型
    pub kind: MessageKind,
    /// 消息内容（JSON 或纯文本）
    pub payload: String,
    /// 发送时间戳（Unix epoch ms）
    pub timestamp: u64,
}

/// 接收方
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Recipient {
    /// 发送给特定窗口
    Single(Uuid),
    /// 广播给所有窗口
    Broadcast,
    /// 发送给一组窗口
    Group(Vec<Uuid>),
    /// 发送给启元自身（特权通道）
    YuanQi,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageKind {
    /// 纯文本聊天
    Text,
    /// 任务请求（请对方协作完成某件事）
    TaskRequest {
        goal: String,
        deadline_ms: Option<u64>,
    },
    /// 任务结果（回应 TaskRequest）
    TaskResult {
        request_id: Uuid,
        success: bool,
        summary: String,
    },
    /// 系统通知（窗口创建/销毁/超时等）
    System,
    /// 工具调用转发
    ToolForward {
        tool: String,
        params: serde_json::Value,
    },
}
```

### 2.3 WindowManager 设计

```rust
// crates/external-bridge/src/window_manager.rs

pub struct WindowManager {
    /// 所有窗口的索引：window_id → Window
    windows: DashMap<Uuid, Arc<RwLock<Window>>>,
    /// 所有者索引：owner → Vec<window_id>（一个 AI 可有多个窗口）
    owner_index: DashMap<String, Vec<Uuid>>,
    /// 消息总线发送端集合（用于广播）
    broadcast_tx: broadcast::Sender<Message>,
    /// 配置
    config: BridgeConfig,
    /// 后台清理任务句柄
    cleanup_handle: tokio::task::JoinHandle<()>,
}

pub struct BridgeConfig {
    /// 窗口默认超时时间（毫秒），超过未活动则挂起
    pub window_timeout_ms: u64,
    /// 每个窗口默认工具调用配额
    pub default_tool_quota: u32,
    /// 每个窗口默认沙箱大小限制（字节）
    pub default_sandbox_size: u64,
    /// 是否启用跨窗口消息审计
    pub audit_messages: bool,
    /// 最大窗口数
    pub max_windows: usize,
}

impl WindowManager {
    /// 创建一个新窗口
    pub async fn create_window(
        &self,
        owner: String,
        requested_permissions: HashSet<ToolPermission>,
    ) -> Result<Arc<RwLock<Window>>, BridgeError> {
        // 1. 宪法引擎检查：该 owner 是否被允许接入
        self.constitution_check(&owner).await?;

        // 2. 权限审批：检查请求的权限是否超出该 owner 的配额
        let approved_permissions = self.approve_permissions(&owner, requested_permissions).await?;

        // 3. 创建独立沙箱目录
        let sandbox_dir = self.create_sandbox(&owner).await?;

        // 4. 创建消息通道
        let (message_tx, message_rx) = mpsc::channel(100);

        // 5. 创建 Window 实例
        let window = Window {
            id: Uuid::new_v4(),
            owner: owner.clone(),
            created_at: Instant::now(),
            last_active: AtomicU64::new(now_ms()),
            permissions: approved_permissions,
            sandbox_dir,
            message_rx,
            message_tx: message_tx.clone(),
            state: Arc::new(RwLock::new(WindowState::Initializing)),
            tool_quota_remaining: AtomicU32::new(self.config.default_tool_quota),
            audit_log_path: audit_path_for(&owner),
        };

        // 6. xray 接线检查：确保新窗口的创建不破坏现有接线
        self.xray_check_new_window(&window).await?;

        // 7. 注册到索引
        let arc_window = Arc::new(RwLock::new(window));
        self.windows.insert(window.id, arc_window.clone());
        self.owner_index.entry(owner).or_default().push(window.id);

        // 8. 状态置为 Active
        *arc_window.write().await.state.write().await = WindowState::Active;

        Ok(arc_window)
    }

    /// 销毁窗口（释放沙箱、关闭通道）
    pub async fn destroy_window(&self, window_id: Uuid) -> Result<(), BridgeError> {
        if let Some(window) = self.windows.get(&window_id) {
            // 1. 状态置为 Destroying
            *window.write().await.state.write().await = WindowState::Destroying;

            // 2. 关闭消息通道
            drop(window.write().await.message_tx.clone());

            // 3. 清理沙箱目录
            let sandbox_dir = window.read().await.sandbox_dir.clone();
            tokio::fs::remove_dir_all(sandbox_dir).await.ok();

            // 4. 从索引中移除
            self.windows.remove(&window_id);

            // 5. 状态置为 Destroyed
            *window.write().await.state.write().await = WindowState::Destroyed;
        }
        Ok(())
    }

    /// 后台清理任务：定期扫描超时窗口
    pub async fn start_cleanup_task(&self) {
        let windows = self.windows.clone();
        let timeout = self.config.window_timeout_ms;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = now_ms();
                for entry in windows.iter() {
                    let window = entry.value();
                    let last_active = window.read().await.last_active.load(Ordering::SeqCst);
                    if now - last_active > timeout {
                        let mut state = window.write().await.state.write().await;
                        if !matches!(*state, WindowState::Destroyed) {
                            *state = WindowState::Suspended {
                                reason: format!("timeout after {}ms", now - last_active),
                            };
                        }
                    }
                }
            }
        });
    }
}
```

### 2.4 MessageBus 设计

```rust
// crates/external-bridge/src/message_bus.rs

impl MessageBus {
    /// 发送消息
    pub async fn send(&self, msg: Message) -> Result<(), BridgeError> {
        // 1. 宪法引擎检查消息内容
        self.constitution_check_text(&msg.payload).await?;

        // 2. 检查发送方权限
        self.check_sender_permission(&msg).await?;

        // 3. 路由
        match &msg.to {
            Recipient::Single(target_id) => {
                if let Some(target) = self.windows.get(target_id) {
                    target.read().await.message_tx.send(msg).await
                        .map_err(|_| BridgeError::WindowClosed)?;
                } else {
                    return Err(BridgeError::WindowNotFound(*target_id));
                }
            }
            Recipient::Broadcast => {
                // 广播给所有窗口（除了发送方自己）
                for entry in self.windows.iter() {
                    if entry.key() != &msg.from {
                        entry.value().read().await.message_tx.send(msg.clone()).await.ok();
                    }
                }
            }
            Recipient::Group(ids) => {
                for id in ids {
                    if let Some(target) = self.windows.get(id) {
                        target.read().await.message_tx.send(msg.clone()).await.ok();
                    }
                }
            }
            Recipient::YuanQi => {
                // 特权通道：发送给启元自身的 LLM 窗口
                self.yuanqi_channel.send(msg).await
                    .map_err(|_| BridgeError::YuanQiUnavailable)?;
            }
        }

        // 4. 审计日志
        if self.config.audit_messages {
            self.append_audit_log(&msg).await?;
        }

        Ok(())
    }

    /// 拉取消息（轮询模式）
    pub async fn poll_messages(
        &self,
        window_id: Uuid,
        since_timestamp: u64,
    ) -> Result<Vec<Message>, BridgeError> {
        // 从窗口的消息队列中取出自 since_timestamp 以来的消息
        // 实现细节：每个窗口维护一个消息缓冲区
        todo!("poll implementation")
    }

    /// SSE 订阅（实时推送模式）
    pub async fn subscribe_sse(
        &self,
        window_id: Uuid,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, BridgeError> {
        // 将窗口的 message_rx 转换为 SSE 流
        todo!("sse implementation")
    }
}
```

### 2.5 ToolProxy 设计

```rust
// crates/external-bridge/src/tool_proxy.rs

pub struct ToolProxy {
    /// 底层工具运行时（复用现有 tool-runtime）
    tool_runtime: Arc<tool_runtime::ToolRuntime>,
    /// 安全闸门
    security_gate: Arc<SecurityGate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub window_id: Uuid,
    pub tool: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub success: bool,
    pub output: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl ToolProxy {
    /// 执行工具调用（核心方法）
    pub async fn call(&self, req: ToolCallRequest) -> Result<ToolCallResponse, BridgeError> {
        let start = Instant::now();

        // 1. 获取窗口
        let window = self.get_window(req.window_id).await?;
        let window_read = window.read().await;

        // 2. 检查窗口状态
        if !matches!(*window_read.state.read().await, WindowState::Active) {
            return Err(BridgeError::WindowNotActive(req.window_id));
        }

        // 3. 检查工具权限
        let tool_perm = string_to_permission(&req.tool)?;
        if !window_read.permissions.contains(&tool_perm) {
            return Err(BridgeError::PermissionDenied {
                window: req.window_id,
                tool: req.tool,
            });
        }

        // 4. 检查配额
        let remaining = window_read.tool_quota_remaining.load(Ordering::SeqCst);
        if remaining == 0 {
            return Err(BridgeError::QuotaExhausted(req.window_id));
        }

        // 5. 安全闸门：宪法引擎检查参数
        self.security_gate.check_tool_params(&req.tool, &req.params).await?;

        // 6. 安全闸门：xray 接线检查（如果涉及写文件）
        if matches!(tool_perm, ToolPermission::WriteFile | ToolPermission::EditFile) {
            self.security_gate.xray_check_write(&req.params).await?;
        }

        // 7. 构造沙箱上下文（注入 window 的 sandbox_dir）
        let sandbox_ctx = SandboxContext {
            root: window_read.sandbox_dir.clone(),
            window_id: req.window_id,
        };

        // 8. 调用底层工具运行时
        let result = self.tool_runtime.execute(&req.tool, &req.params, &sandbox_ctx).await
            .map_err(|e| BridgeError::ToolExecutionFailed(e.to_string()))?;

        // 9. 扣减配额
        window_read.tool_quota_remaining.fetch_sub(1, Ordering::SeqCst);

        // 10. 审计日志
        self.audit_tool_call(req.window_id, &req.tool, &result, start.elapsed()).await?;

        Ok(ToolCallResponse {
            success: result.exit_code == 0,
            output: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
```

### 2.6 SecurityGate 设计

```rust
// crates/external-bridge/src/security_gate.rs

pub struct SecurityGate {
    /// 宪法引擎（复用现有 constitution.rs）
    constitution: Arc<ConstitutionEngine>,
    /// xray 接线检查器（复用现有 project-xray）
    xray: Arc<XrayChecker>,
    /// 沙箱策略（复用现有 sandbox）
    sandbox: Arc<SandboxPolicy>,
    /// 危险命令黑名单
    dangerous_patterns: Vec<regex::Regex>,
}

impl SecurityGate {
    /// 检查工具参数是否违反宪法
    pub async fn check_tool_params(
        &self,
        tool: &str,
        params: &serde_json::Value,
    ) -> Result<(), BridgeError> {
        // 1. 将参数序列化为文本
        let text = serde_json::to_string(params)?;

        // 2. 危险模式检查
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(&text) {
                return Err(BridgeError::DangerousPattern {
                    tool: tool.to_string(),
                    pattern: pattern.as_str().to_string(),
                });
            }
        }

        // 3. 宪法引擎检查
        self.constitution.check_text(&text).await?;

        Ok(())
    }

    /// xray 接线检查（写文件前）
    pub async fn xray_check_write(
        &self,
        params: &serde_json::Value,
    ) -> Result<(), BridgeError> {
        // 提取目标文件路径
        let target_path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or(BridgeError::MissingPath)?;

        // 运行 xray scan，检查写入该文件是否会破坏接线
        self.xray.scan_path(target_path).await?;

        Ok(())
    }

    /// 创建窗口时检查所有者是否被允许接入
    pub async fn check_owner_admission(&self, owner: &str) -> Result<(), BridgeError> {
        // 1. 黑名单检查
        if self.constitution.is_owner_banned(owner).await {
            return Err(BridgeError::OwnerBanned(owner.to_string()));
        }

        // 2. 宪法检查
        self.constitution.check_owner(owner).await?;

        Ok(())
    }
}
```

---

## 3. API 设计

### 3.1 REST API（面向不支持 MCP 的客户端）

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/bridge/window` | 创建新窗口 |
| `DELETE` | `/bridge/window/:wid` | 销毁窗口 |
| `GET` | `/bridge/window/:wid/status` | 查询窗口状态 |
| `GET` | `/bridge/window/:wid/tools` | 列出可用工具 |
| `POST` | `/bridge/window/:wid/tool` | 调用工具 |
| `POST` | `/bridge/window/:wid/message` | 发送消息 |
| `GET` | `/bridge/window/:wid/messages?since=:ts` | 拉取消息 |
| `GET` | `/bridge/window/:wid/messages/stream` | SSE 实时推送 |
| `GET` | `/bridge/windows` | 列出所有窗口（管理员） |
| `POST` | `/bridge/window/:wid/approve` | 人工审批（高风险操作） |

### 3.2 MCP Server 模式（面向 MCP 兼容 Host）

External Bridge 同时运行一个 MCP Server，暴露以下 MCP 原语：

**Tools（暴露给外部 AI 调用）**：
- `bridge_create_window` — 创建窗口（通常由 Host 自动调用）
- `bridge_call_tool` — 调用启元身体的工具
- `bridge_send_message` — 发送消息给另一个窗口
- `bridge_poll_messages` — 拉取待处理消息
- `bridge_get_status` — 查询窗口状态

**Resources（暴露给外部 AI 读取）**：
- `bridge://windows` — 所有窗口列表
- `bridge://window/:wid/state` — 特定窗口状态
- `bridge://audit/:wid` — 特定窗口的审计日志

**Prompts（预设提示模板）**：
- `bridge_init` — 窗口初始化提示
- `bridge_collaborate` — 协作请求模板

### 3.3 内部 Rust API（面向同为 Rust 生态的客户端）

```rust
// 直接 use，无需 HTTP/MCP 开销
use external_bridge::{WindowManager, MessageBus, ToolProxy, SecurityGate};

let bridge = ExternalBridge::new(config).await?;

// 创建窗口
let window = bridge.create_window("yuanbao".to_string(), permissions).await?;

// 调用工具
let result = bridge.call_tool(window.id, "read_file", json!({"path": "/tmp/test.rs"})).await?;

// 发送消息
bridge.send_message(Message {
    from: window.id,
    to: Recipient::Single(other_window_id),
    kind: MessageKind::TaskRequest { goal: "...".to_string(), deadline_ms: Some(60000) },
    payload: "帮我审查这个模块".to_string(),
    ..Default::default()
}).await?;
```

---

## 4. 安全设计

### 4.1 四层防御（对应 G0-G3）

| 层 | 机制 | 实现位置 |
|----|------|---------|
| **G0 物理层** | landlock + seccomp 沙箱，每个窗口独立 | `crates/sandbox` |
| **G1 反射层** | xray 接线断言，写文件前检查 | `crates/project-xray` |
| **G2 案例层** | 危险模式正则 + 操作审计 | `SecurityGate` |
| **G3 叙事层** | 宪法引擎文本审查 | `crates/agent-core/src/constitution.rs` |

### 4.2 窗口隔离保证

| 隔离维度 | 机制 |
|---------|------|
| 文件系统 | 每个窗口独立 `sandbox_dir`，landlock 绑定 |
| 内存空间 | 每个窗口独立 `Arc<RwLock<Window>>`，无共享可变状态 |
| 消息通道 | 每个窗口独立 `mpsc::Receiver`，不交叉 |
| 工具配额 | 每个窗口独立 `AtomicU32` 计数器 |
| 审计日志 | 每个窗口独立日志文件 |

### 4.3 危险命令黑名单（默认）

```rust
const DANGEROUS_PATTERNS: &[&str] = &[
    r"rm\s+-rf\s+/",
    r"mkfs\.",
    r"dd\s+if=.*of=/dev/",
    r":\(\)\s*\{.*\};:",  // fork bomb
    r"chmod\s+-R\s+777\s+/",
    r">\s*/dev/sda",
    r"shutdown|reboot|halt|poweroff",
    r"curl.*\|.*sh",
    r"wget.*\|.*sh",
];
```

### 4.4 人工审批门

对于以下操作，必须经过人工审批（返回 `202 Accepted`，等待 `/bridge/window/:wid/approve`）：

| 操作 | 触发条件 |
|------|---------|
| 写入 `/etc/` 或 `/usr/` | 系统目录保护 |
| 网络外联 | 非白名单域名 |
| `tool_quota` 超额 | 需要管理员提升配额 |
| 跨窗口消息含敏感词 | 宪法引擎标记 |
| 窗口创建请求来自未注册 owner | 新 AI 个体首次接入 |

---

## 5. 与现有组件的集成

| 现有组件 | 集成方式 | 改动量 |
|---------|---------|--------|
| `service`（axum 路由） | 新增 `/bridge/*` 路由组 | 小（新增文件 `routes_bridge.rs`） |
| `agent-core`（loop.rs） | 不直接修改；bridge 作为独立 crate 调用 tool-runtime | 零 |
| `tool-runtime` | 复用现有工具执行引擎，新增 `SandboxContext` 参数 | 中（需要支持沙箱路径注入） |
| `project-xray` | 调用 `xray::scan_path()` 做写前检查 | 小（新增一个公共函数） |
| `constitution` | 调用 `constitution::check_text()` 做文本审查 | 零 |
| `sandbox`（landlock/seccomp） | 为每个窗口创建独立沙箱实例 | 中 |
| `experience` | 可选：窗口操作记录写入经验库 | 小 |
| `memory` | 可选：窗口上下文存入记忆系统 | 小 |

---

## 6. 开发阶段与验收标准

### Phase 1：核心骨架（v16.0，预计 3-4 天）

**交付物**：
- `crates/external-bridge` 基础结构（Cargo.toml + lib.rs + 模块划分）
- `WindowManager`：create / destroy / query / timeout
- `MessageBus`：point-to-point + broadcast
- 基础 REST API（axum 路由）

**验收标准**：
- ✅ 能创建 3 个窗口，每个窗口独立收发消息
- ✅ 窗口超时后自动挂起
- ✅ `cargo test` 在 `crates/external-bridge` 下 ≥ 10 个测试通过
- ✅ xray 断言：bridge crate 存在且接线正确

### Phase 2：工具代理 + 安全闸门（v16.1，预计 3-4 天）

**交付物**：
- `ToolProxy`：调用 tool-runtime，注入沙箱上下文
- `SecurityGate`：宪法检查 + xray 检查 + 危险模式匹配
- 配额管理（AtomicU32 计数器）
- 审计日志（JSONL 格式）

**验收标准**：
- ✅ 窗口 A 调用 `read_file` 只能读取自己的 `sandbox_dir`
- ✅ 窗口 A 无法调用未授权的工具（返回 `PermissionDenied`）
- ✅ 危险命令（如 `rm -rf /`）被拦截，返回 `DangerousPattern`
- ✅ 审计日志记录每次工具调用（JSONL 格式）
- ✅ xray 断言：新增 `bridge-tool-proxy-wired`

### Phase 3：MCP Server 模式（v16.2，预计 2-3 天）

**交付物**：
- MCP Server 实现（基于 `rmcp` crate 或手写 JSON-RPC over stdio）
- 暴露 Tools / Resources / Prompts
- 与 Claude Desktop / Codex CLI 的集成测试

**验收标准**：
- ✅ Claude Desktop 配置后，能发现 `bridge_*` 工具
- ✅ 通过 MCP 调用 `bridge_call_tool` 成功执行 `read_file`
- ✅ 跨窗口消息通过 MCP 正常传递
- ✅ xray 断言：MCP Server 二进制存在且可启动

### Phase 4：高级功能（v17+，预计 2-3 天）

**交付物**：
- SSE 实时消息推送（替代轮询）
- 窗口状态持久化（重启后恢复）
- 人工审批门（Web UI 或 CLI）
- 窗口间任务协作（TaskRequest / TaskResult 完整流程）

**验收标准**：
- ✅ SSE 流在消息到达时 < 100ms 推送到客户端
- ✅ `kill -9` 后重启，窗口状态恢复（Active/Suspended 正确）
- ✅ 高风险操作返回 202，审批后继续执行
- ✅ 两个窗口通过 TaskRequest/TaskResult 完成一次协作

---

## 7. 文件结构

```
crates/external-bridge/
├── Cargo.toml
├── src/
│   ├── lib.rs              # 入口，导出公共 API
│   ├── window.rs          # Window 数据结构 + WindowState
│   ├── window_manager.rs  # 窗口生命周期管理
│   ├── message_bus.rs     # 消息路由 + 广播
│   ├── tool_proxy.rs      # 工具代理 + 配额管理
│   ├── security_gate.rs   # 四层安全闸门
│   ├── mcp_server.rs      # MCP 协议适配层
│   ├── rest_api.rs       # REST + SSE 路由
│   ├── config.rs          # BridgeConfig 定义
│   ├── audit.rs           # 审计日志（JSONL）
│   └── error.rs           # BridgeError 定义
├── tests/
│   ├── test_window_lifecycle.rs
│   ├── test_message_bus.rs
│   ├── test_tool_proxy.rs
│   ├── test_security_gate.rs
│   └── test_mcp_integration.rs
└── CONTRACT.md            # 契约文档（Public Interface / Invariants / Wiring）
```

---

## 8. CONTRACT.md（契约文档）

```markdown
# external-bridge CONTRACT

## Public Interface

### WindowManager
- `create_window(owner, permissions) -> Result<Arc<RwLock<Window>>>`
- `destroy_window(window_id) -> Result<()>`
- `get_window(window_id) -> Result<Arc<RwLock<Window>>>`
- `list_windows() -> Vec<WindowSummary>`
- `start_cleanup_task()`

### MessageBus
- `send(msg: Message) -> Result<()>`
- `poll_messages(window_id, since) -> Result<Vec<Message>>`
- `subscribe_sse(window_id) -> Result<Sse<Stream>>`

### ToolProxy
- `call(req: ToolCallRequest) -> Result<ToolCallResponse>`

### SecurityGate
- `check_tool_params(tool, params) -> Result<()>`
- `xray_check_write(params) -> Result<()>`
- `check_owner_admission(owner) -> Result<()>`

## Invariants（核心不变量）

1. **窗口隔离**：窗口 A 的 `sandbox_dir` 永远不被窗口 B 访问
2. **权限不可提升**：窗口创建后，permissions 只能减少，不能增加
3. **审计不可绕过**：每次工具调用必须写入审计日志，无例外
4. **宪法不可绕过**：所有文本（消息内容 + 工具参数）必须经过宪法检查
5. **配额不可透支**：tool_quota_remaining 为 0 时，工具调用必须拒绝
6. **消息不丢失**：发送成功 = 接收方已确认（at-least-once 语义）

## Wiring（xray 断言 ID 列表）

- `bridge-crate-exists` — external-bridge crate 存在
- `bridge-tool-proxy-wired` — ToolProxy 调用 tool-runtime
- `bridge-security-gate-wired` — SecurityGate 调用 constitution + xray
- `bridge-window-isolation` — 窗口 sandbox_dir 独立
- `bridge-audit-logged` — 每次工具调用写入审计日志
- `bridge-quota-enforced` — 配额耗尽时拒绝调用
```

---

## 9. 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| 窗口数爆炸导致内存耗尽 | `max_windows` 硬限制 + 超时自动回收 |
| 消息总线成为瓶颈 | 每窗口独立 mpsc 通道，无全局锁竞争 |
| 沙箱逃逸 | landlock + seccomp 双重限制，定期审计 |
| 外部 AI 伪造身份 | owner 认证（API key 或 mTLS），宪法引擎验证 |
| 消息内容绕过宪法 | 所有消息在发送前必经 SecurityGate |
| 配额机制被绕过 | AtomicU32 原子操作，无锁竞争 |
| xray 断言被绕过 | 子串匹配 + CI 第四门强制检查 |
| MCP 协议升级不兼容 | 版本协商 + 向后兼容层 |

---

## 10. 不做的事（Non-Goals）

以下不在本 RFC 范围内，推迟到 v17+：

- ❌ **窗口间自动任务分配**：需要复杂的调度算法，v17 再设计
- ❌ **跨窗口共享内存**：安全模型过于复杂，初期只支持消息传递
- ❌ **外部 AI 的自动注册**：所有窗口创建需人工审批
- ❌ **窗口的 GPU 资源配额**：当前所有工具都是 CPU 密集型
- ❌ **多节点部署**：当前设计假设单进程内运行
- ❌ **WebSocket 支持**：v16 只支持 SSE，WebSocket 留待 v17

---

## 11. 开放问题（待评估阶段讨论）

| 问题 | 选项 | 倾向 |
|------|------|------|
| MCP 实现用 `rmcp` crate 还是手写？ | A. rmcp（成熟但依赖重） B. 手写 JSON-RPC（轻量但工作量大） | B（保持 VM 无 github 的约束） |
| 消息持久化用 JSONL 还是 SQLite？ | A. JSONL（简单，已有经验） B. SQLite（查询强但引入依赖） | A（单用户场景，JSONL 够用） |
| 窗口配额默认值多少？ | A. 100 次/窗口 B. 1000 次/窗口 C. 无限制 | B（够用且可控） |
| 是否需要窗口优先级？ | A. 启元自身 LLM 优先 B. 所有窗口平等 C. 按 owner 分级 | C（管理员可配置） |
| SSE 还是 WebSocket？ | A. SSE（单向推送，简单） B. WebSocket（双向，复杂） | A（v16 只需单向） |

---

## 12. 评审检查清单

请评审者逐条确认：

- [ ] 窗口隔离机制是否足够强（landlock + seccomp 覆盖所有逃逸路径？）
- [ ] 消息总线的 at-least-once 语义是否会导致重复消息问题？
- [ ] ToolProxy 的配额机制是否可能被并发请求绕过？
- [ ] SecurityGate 的危险模式正则是否覆盖了实际威胁？
- [ ] MCP Server 的 stdio 通信是否会阻塞主线程？
- [ ] 窗口超时回收是否会误杀活跃会话？
- [ ] 审计日志的写入是否会成为性能瓶颈？
- [ ] CONTRACT.md 的不变量是否全部可被 xray 断言覆盖？
- [ ] Phase 1-4 的验收标准是否可量化、可自动化？
- [ ] 不做的事（Non-Goals）是否有遗漏的重要功能？

---

**文档结束。请评估。**
