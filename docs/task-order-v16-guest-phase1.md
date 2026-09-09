# 任务书：v16 Guest Session Phase 1（施工窗口执行）

- **下达**：顶层架构角色，2026-07-31
- **版本**：**v1.1（施工前提核验后修订）**。v1.0 有 4 处缺口，见文末 §7 修订记录。**以本版为准。**
- **规格来源**：`docs/rfc-guest-session-v3.md`（RFC-003，已 Accept）+ `docs/reply-to-yuanbao-rfc003.md`（4 处修正案 + 5 项决策，起草方已全部确认）
- **执行角色**：施工窗口。本任务书自足，照做即可；与 RFC 冲突处**以本任务书为准**。
- **施工基线**：主干 v17.0 `fd5829e`（**注意：RFC 评审基线是 v14.0，主干已前进三轮**。§2 依赖核验已在 v17.0 现场复验）。
- **预算**：2-3 天。完成后按 §5 验收，真 Linux 门禁全绿后交验。

---

## §1 一句话目标

外部 AI 经 REST 创建受审批的只读 guest 会话，在独立 workspace 内只能调用 `read/glob/grep` 三个工具，全程审计落盘。

## §2 交付物清单

| # | 文件 | 动作 | 内容 |
|---|------|------|------|
| 1 | `crates/service/src/guest.rs` | 新增 ~200 行 | `GuestSession` + create/approve/destroy/call_tool + 审计追加 + `build_guest_dispatcher()` |
| 2 | `crates/service/src/guest_approval.rs` | 新增 ~80 行 | 审批队列（内存 HashMap）+ `require_auth_for_approval` 中间件 |
| 3 | `crates/service/src/lib.rs` | +2 行 | `mod guest; mod guest_approval;`（**不要**在 session.rs 里声明） |
| 4 | `crates/service/src/routes.rs` | +~100 行 | 6 个 handler（见 §3.1 路由表） |
| 5 | `crates/service/src/main.rs` | +~6 行 | 路由挂载，追加在现有 `.route(...)` 链（main.rs:683-699 附近） |
| 6 | `docs/xray/wiring-v13.toml` | 追加 4 条断言 | **注意：追加到这个文件，不要新建 wiring-v16.toml**——xray CLI 默认 spec 路径硬编码为它（project-xray/src/main.rs:36），新建文件 CI 不会跑。顺带把该文件头注释的"7 条"改成实际条数 |
| 7 | `.gitignore` | **新建**（仓库根当前确实没有，已核验） | 至少含：`.guest_workspaces/`、`target/`、`.workbuddy/` |
| 8 | `crates/service/CONTRACT.md` | 新增 | 按 RFC-003 §8 全文落盘。**验收第 14 条查它** |
| 9 | 单元/集成测试 | 新增 | 见 §5 验收 10/11/12 |

### 2.1 依赖核验（v17.0 现场复验，**区分正式依赖与 dev 依赖**）

| 依赖 | 位置 | 生产代码可用？ |
|---|---|---|
| `axum` / `tokio` / `serde` / `serde_json` / `uuid` / `chrono` / `tower-http` | `[dependencies]` | ✅ 可用 |
| `tools-builtin`(:20) / `tool-runtime`(:19) / `agent-types`(:12) | `[dependencies]` | ✅ 可用 → `ReadTool::new()` 等可直接构造 |
| **`tempfile`(:51) / `tower`(:47)** | **`[dev-dependencies]`** | ❌ **仅测试可用** |

**由此产生一条硬约束**：`guest.rs` / `guest_approval.rs` 的**生产代码禁止使用 `tempfile`**。
workspace 目录用 `std::fs::create_dir_all(&workspace_root)` 创建即可——路径本来就是确定的 `./.guest_workspaces/<uuid>/`，不需要临时目录语义。`tempfile` 只允许出现在 `#[cfg(test)]` 或 `tests/` 下。

**结论：不需要新增任何依赖。** 若你认为必须新增，停下来上报，不要自行添加。

## §3 关键实现约束（修正案，必须照抄）

### 3.1 路由表（6 条）

| 方法 | 路径 | 认证 |
|------|------|------|
| POST | `/api/v1/guest/session` | 无（label 自声明），创建即 `pending_approval` |
| GET | `/api/v1/guest/session/:id` | `Authorization: Bearer <guest_token>` |
| DELETE | `/api/v1/guest/session/:id` | guest_token |
| POST | `/api/v1/guest/session/:id/tool` | guest_token，且 status==Active |
| POST | `/api/v1/guest/session/:id/audit` | guest_token |
| POST | `/api/v1/guest/approve/:id` | **`require_auth_for_approval`：api_key/UserStore 未配置 → 503，绝不 pass-through** |

### 3.2 dispatcher：白名单直接构造（禁止黑名单复用）

```rust
// crates/service/src/guest.rs
const GUEST_TOOL_NAMES: &[&str] = &["read", "glob", "grep"];

fn build_guest_dispatcher() -> ToolDispatcher {
    let mut d = ToolDispatcher::new();
    d.register(Arc::new(ReadTool::new()));
    d.register(Arc::new(GlobTool::new()));
    d.register(Arc::new(GrepTool::new()));
    d
}
```

- 真名注意：类型是 `ToolDispatcher`（tool-runtime/dispatcher.rs:50）；`write_file` 工具住在 `edit.rs`。
- **禁止**调用 `read_only_view()`；**禁止**经 `tool_runtime::registry`（它没有 `get`，是工具市场）。
- `GUEST_TOOL_NAMES` 常量的**字面量必须与上面逐字一致**（含空格），xray 断言按完整字面量锁。

### 3.3 隔离唯一键：ctx.cwd

- 每个 guest 会话：`workspace_root = ./.guest_workspaces/<uuid>/`。
- 调用工具时构造的 `ToolContext.cwd` **必须**赋值为该 `workspace_root`——这是 guest 唯一隔离防线（read.rs 的 strip_prefix 校验以 cwd 为界），不许有第二条构造路径。

### 3.4 数据类型

- 时间一律 `chrono::DateTime<Utc>`（禁 `Instant`）。
- `guest_token` 独立于 session id 生成（`Uuid::new_v4()` 字符串即可），只经 `Authorization` header 传输，不出现在 URL 与日志。
- 配额：`tool_count` 上限 1000，超限 429；TTL 默认 24h，过期 status=Expired。
- 审计 JSONL：`workspace_root/.guest_audit/<guest_id>.jsonl`，字段按 RFC-003 §4.5。

### 3.4b 并发语义：label **不是**唯一键（回函 §3 决策 3，v1.0 漏落）

- `label` 是**自声明的展示名**，**不做唯一性约束、不做去重、不做冲突拒绝**。
- 同一个 `label` 允许同时存在**多个** guest 会话，每个会话：
  - 有自己的 `guest_id`、`guest_token`、`workspace_root`；
  - **各自独立走审批**——批准其中一个，不影响其余仍处于 `pending_approval`。
- 会话表的唯一键是 `guest_id`（`Uuid`），**禁止**用 `HashMap<label, Session>` 这类以 label 为键的结构。

### 3.4c workspace 不得进入 git 与打包（回函 §3 决策 5，第二条 v1.0 漏落）

两条都要做，只做 `.gitignore` 不算完成：

1. **git**：新建 `.gitignore`，含 `.guest_workspaces/`（见 §2 交付物 #7）。
2. **打包/同步**：项目标准流程是 tar 上传真 Linux VM（当前排除 `target/`、`.workbuddy/`、`.git`）。**`.guest_workspaces/` 必须并列加入排除清单。**
   - 若排除清单固化在脚本里，改脚本；若是临时命令行拼装，把新的完整命令写进交付物说明，供后续复用。
   - 理由：guest 工作区含审计日志与外部 AI 的读取快照，进 git 历史或上传包等于把它们泄露出去。

### 3.5 追加到 `docs/xray/wiring-v13.toml` 的 4 条断言（severity 全部 red）

**字面量锁的可行性已实测**（施工前提核验，v17.0 现场）：

| 核验项 | 结论 |
|---|---|
| 引擎字段 | 只有 `any`(OR) / `all`(AND)，判定 `content.contains`（`wiring.rs:41-53`、`:135-136`）→ **无否定式**，正向锁是唯一路径 |
| TOML 单引号内含双引号 | ✅ 可解析，字面量原样保留（已用真 TOML 解析器实测） |
| rustfmt 会不会断锁 | ✅ 仓库无自定义 `rustfmt.toml`，默认 `max_width=100`；锁定行 61 字符，**不会被换行** |
| `ToolContext.cwd` 字段 | ✅ 真实存在（`tool-runtime/src/context.rs:9`） |
| 风格 | 现有 11 条断言的 `[[capability.chain]]` 一律**顶格**，下方写法已对齐。**这是本仓首次使用单引号字面量锁**，注册后请确认 `codex-xray wiring` 能正常读取该条 |

```toml
[[capability]]
id = "guest-tools-are-readonly"
claim = "guest 白名单只含 read/glob/grep"
severity = "red"

[[capability.chain]]
file = "crates/service/src/guest.rs"
all = ['const GUEST_TOOL_NAMES: &[&str] = &["read", "glob", "grep"];']
meaning = "白名单字面量锁：任何增删改都会使断言变红"

[[capability]]
id = "guest-dispatcher-is-whitelist"
claim = "guest dispatcher 由白名单直接构造"
severity = "red"

[[capability.chain]]
file = "crates/service/src/guest.rs"
all = ["fn build_guest_dispatcher", "ReadTool::new", "GlobTool::new", "GrepTool::new"]
meaning = "逐工具点名注册，不经黑名单视图"

[[capability]]
id = "guest-ctx-cwd-is-workspace-root"
claim = "guest 工具调用的 cwd 强制等于 workspace_root"
severity = "red"

[[capability.chain]]
file = "crates/service/src/guest.rs"
all = ["cwd: session.workspace_root.clone()"]
meaning = "隔离唯一键；施工时保证源码里恰好是这个写法"

[[capability]]
id = "guest-approval-auth-required"
claim = "审批端点在无认证配置时不可用"
severity = "red"

[[capability.chain]]
file = "crates/service/src/guest_approval.rs"
all = ["fn require_auth_for_approval", "SERVICE_UNAVAILABLE"]
meaning = "api_key 缺失时 503，不 pass-through"
```

**施工时必须保证 `guest.rs` 里的白名单常量与锁定字面量逐字一致**（含空格、含分号）：

```rust
const GUEST_TOOL_NAMES: &[&str] = &["read", "glob", "grep"];
```

（第 3 条的锁定子串若与实际实现写法不符，允许微调子串，但**必须**同步保证断言绿且语义不变；不许删条。第 1 条的字面量锁**不允许**微调——微调等于把锁配到自己的实现上，锁就失去意义。）

## §4 红线

1. 只动 §2 清单里的文件。**不碰** loop.rs / dispatcher.rs / sandbox / tools-builtin / experience。
2. 不新增 crate、不新增外部依赖、不顺手重构、不顺手修别的债（包括硬编码 key——那是另一份任务书）。
3. `read_only_view()` 三个字不允许出现在 guest.rs / guest_approval.rs（测试代码里的否定断言除外）。

## §5 验收标准（16 条，逐条出证据）

1. `POST /guest/session` → `pending_approval`；审批前任何工具调用 403。
2. `approve` 无 Bearer token → 401；api_key/UserStore 未配置 → 503。
3. 审批后 read/glob/grep 正常返回。
4. `write_file`/`bash`（白名单外）→ 403。
5. guest 读 `../` 或 workspace 外绝对路径 → 拒绝（read.rs 校验生效证明）。
6. 审计 JSONL 存在、每次调用追加、格式可解析。
7. token 过期（可用短 TTL 测试）后调用 → 403/410。
8. 超 1000 次调用 → 429。
9. xray 4 条新断言注册于 wiring-v13.toml 且 `codex-xray wiring` 全绿。
10. 单测：`guest_dispatcher` 工具名集合 == `{read, glob, grep}`。
11. 集成测试：向 guest dispatcher 额外注册一个 fake 工具后，经 guest 调用返回 403/未注册（防黑名单回归）。
12. CI grep 步骤：`grep -c read_only_view crates/service/src/guest.rs` == 0。
13. 真 Linux 三门 + xray 第四门全绿（fmt / clippy -D warnings / test / wiring），按项目标准流程：tar 上传 → touch 刷 mtime → 前台跑门禁 → 判读 gate 日志。
14. **`crates/service/CONTRACT.md` 已落盘**，内容覆盖 RFC-003 §8 全部条款（附文件路径 + 行数）。
15. **同 label 并发**：用同一个 `label` 连续创建 2 个会话 → 返回两个不同 `guest_id`，均为 `pending_approval`；只批准其中一个 → 另一个仍 `pending_approval` 且工具调用 403。
16. **`.guest_workspaces/` 双重排除**：① `git status` 在 guest 会话跑过之后仍干净（`.gitignore` 生效）；② 上传 VM 的 tar 排除清单含 `.guest_workspaces`（附实际命令或脚本 diff）。

## §6 交验物

- 改动文件清单 + 每条验收标准的证据（命令输出/测试名/日志行）。
- 不要写"已完成"三个字而不附证据——验收按"不信报告信源码"执行。

---

## §7 v1.1 修订记录（施工前提核验后补，2026-07-31）

v1.0 下发前未做施工前提现场核验，事后核验抓到 4 处缺口，全部已在本版修补：

| # | 缺口 | 性质 | 本版修补处 |
|---|---|---|---|
| 1 | 把 `tempfile`/`tower` 这两个 **dev-dependency** 说成"已有依赖"，同时又禁止施工窗口自行加依赖 | **会卡死施工**：生产代码用 tempfile → 编译失败且被禁自救 | §2.1 依赖表区分正式/dev + 明确用 `std::fs::create_dir_all` |
| 2 | 回函 §3 决策 3「同 label 多并发，各自独立审批」**完全未落** | 共识丢失：可能实现成 label 唯一键 | §3.4b + 验收 15 |
| 3 | 回函 §3 决策 5 第二条「打包脚本排除清单同步加」**未落**（只落了 `.gitignore`） | 泄露风险：§5-13 要求 tar 上传 VM，审计日志会被打包带走 | §3.4c + 验收 16 |
| 4 | `CONTRACT.md` 列在交付物清单，但 13 条验收无对应条目 | 交付物无验收 = 可以不交 | 验收 14 |

另做 3 处非缺口修正：断言块缩进对齐现有文件风格（顶格）、补充字面量锁的可行性实测结论（TOML 可解析 / rustfmt 不断锁 / `ToolContext.cwd` 存在）、标注施工基线已从 RFC 评审时的 v14.0 前进到 v17.0。

**教训（与三轮 RFC 同源）**：缺口 2、3 是**回函里已达成共识、抄进任务书时漏抄**。三轮 RFC 反复抓的"表格里每一格都要核，不只是重点格"，这次犯在自己手上——**回函到任务书的转写，必须逐条打勾，不能凭印象**。
