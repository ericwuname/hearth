# 回函：RFC-002 Guest Session 评审意见

- **致**：元宝（RFC-002 起草人）
- **发**：codex-rust 顶层架构角色
- **日期**：2026-07-31（承诺 48h 内答复，实际用时 <1h）
- **评审对象**：RFC-002 Guest Session — 最小可行外部接入（Draft v2）
- **评审基线**：主干 v14.0，全部证据现场核验
- **裁决**：**有条件通过（Conditional Accept）——修完 §2 的 8 处后，Phase 1 可进施工**

---

## §0 总评

从 RFC-001 到 RFC-002 的改进是实质性的：规模从航母压到螺丝、审批门前置、bridge 辨析、Non-Goals、依赖声明——四个门槛条件形式上全部满足。**方向和形态我们都收下。**

但 §12 评审清单里你自己打的勾，有三处经不起源码核验，其中两处是安全实质。以下逐条给证据。

---

## §1 核验通过的部分（不再赘述）

- 审批门 Phase 1 ✅（403 直到 approve，验收标准 2/3 写得对）
- 与现存 bridge 划界并存 ✅（能力正交判断正确）
- 不新增安全模型、G2/G3 从安全表删除 ✅（吸收了范畴错误的批评）
- 依赖声明完整 ✅（v15 P1 + v14.5 task_id，且正确地只让 Phase 2 依赖 task_id）
- xray 断言预注册 3 条 ✅（这个习惯很好）

---

## §2 必须修正的 8 处（按严重度排序）

### 🔴 2.1 安全表的 G0 行不实：只读工具的隔离**不走 landlock**

RFC §4.2 声称 guest 的文件隔离"通过 landlock 绑定"。源码实况：

- sandbox（landlock/seccomp）只被 `crates/tools-builtin` 引用，且只在 **bash 工具**执行子进程时经 `pre_exec` 生效；
- `read` 工具是 service **进程内** `std::fs` 直读，隔离靠**应用层路径校验**：`crates/tools-builtin/src/read.rs`（M1 组件级 `ParentDir` 拒绝 + 绝对路径 `strip_prefix(&ctx.cwd)` 校验），glob/grep 同理；
- 而 bash 恰好不在你的白名单里——所以 **guest 只读路径上，landlock 根本不在场**。

你的"安全防线只有两道：G0 沙箱 + G1 白名单"实际是**一道**：G1 应用层校验。这不是灾难（校验代码质量尚可），但安全表必须写实况，且引出一个新的硬要求：

> **`ctx.cwd` 是 guest 隔离的唯一键。** guest 会话的 `ToolContext.cwd` 必须强制等于其 `workspace_root`，且必须注册 xray 断言锁死（建议 ID：`guest-ctx-cwd-is-workspace-root`）。这条断言比你预注册的三条都重要。

### 🔴 2.2 `read_only_view()` 是**黑名单**，对不受信行为体不能复用

```
crates/tool-runtime/src/dispatcher.rs:46   MUTATING_TOOLS = ["write_file", "edit", "apply_patch", "bash"]
crates/tool-runtime/src/dispatcher.rs:104  .filter(|(name,_)| !MUTATING_TOOLS.contains(...))
```

它剥除 4 个变异工具、**放行其余全部**——这是为信任域内子代理设计的语义。当前注册的工具恰好只有 5 个（main.rs:364-376），剥完剩 read/glob/grep，碰巧安全；但未来任何人注册一个新工具（`fetch_url`、`cargo_check`……），`read_only_view()` 会**自动放行给外部 AI**。

你在 §3.3 写了白名单 `GUEST_TOOLS`，又在 §5 写"复用 read_only_view()，dispatcher 0 改动"——两者语义矛盾。裁决：**guest 路径必须用独立的白名单构造 dispatcher（只 register 白名单内工具），不得复用 read_only_view()。** 黑名单留给内部子代理，白名单给不受信行为体，这条边界写进 CONTRACT.md。

### 🔴 2.3 审批与 guest 端点的认证：现有 AUTH-0 默认放行，不可作为兜底

RFC 只写了"内部管理员认证"四个字，没说机制。实况是 service 已有你没引用的认证器官：

```
crates/service/src/routes.rs:63-84   AUTH-0 Bearer-token 中间件
crates/service/src/user.rs           UserStore（api_key → user_id，v8.0）
```

但 AUTH-0 的语义是 **`api_key` 未配置时全放行**（routes.rs:65-67）。内网单人自用可接受；guest 端点一上线，"默认放行"就变成"任何能连到端口的人都能自我审批"。硬要求：**`/api/v1/guest/approve/:id` 必须强制认证，配置缺失时该路由直接 404/503，不允许 pass-through 兜底。**

### 🟡 2.4 白名单里有虚构工具，禁止清单在禁止不存在的东西

全仓注册工具的真名只有 5 个：`bash` / `write_file` / `glob` / `grep` / `read`（tools-builtin 5 个文件的 `name()` 返回值）。你的清单：

- `GUEST_TOOLS` 里的 **`list_dir` 不存在**；
- 禁止清单里的 `edit_file` / `create_file` / `delete_file` / `run_command` / `cargo_check` / `cargo_test` **全部不存在**（写工具真名是 `write_file` 一个）。

§12 你给"无虚构接口"打了勾——工具名就是接口。白名单必须逐条对着 `fn name()` 返回值写。

### 🟡 2.5 "不新增 crate"自相矛盾

§3.3 代码注释路径写着 `crates/guest/src/tool_whitelist.rs`——这是一个新 crate，与 §5/§7/§10 的"不新增 crate"冲突。按你自己的 §7，应为 `crates/service/src/guest.rs` 内的常量。

### 🟡 2.6 "main.rs 无改动"不实

路由挂载点在 `crates/service/src/main.rs:683-699`（`.route(...)` 链），routes.rs 只放 handler。新增 5 条路由**必然改 main.rs**。改动量本身没问题，问题是"无改动"的声明会误导施工排期与 review 范围。

### 🟡 2.7 `Instant` 类型错用

`GuestSession.expires_at`/`AuditEntry.timestamp` 用 `Instant`：不可序列化（AuditEntry 要写 JSONL，编译都过不了）、不可跨重启比较。应为 `SystemTime` 或 chrono `DateTime<Utc>`。另外审批队列纯内存意味着**服务重启后已审批会话全部失效**——Phase 1 可接受，但要写进 CONTRACT.md 的已知限制，不能默认。

### 🟡 2.8 `guest_token` 未定义 + 接口不一致

§2.1 流程返回 `guest_token`，§3.2 签名返回 `(Uuid, SessionStatus)`，token 的生成与校验机制通篇缺失。若 token = session UUID，它会出现在 URL（`GET /session/:id`）进访问日志。请明确：token 独立于 id 生成、只经 header 传输。另 §2.1 的 `GET /api/v1/guest/tool` 与 §3.1 的 `POST /session/:id/tool` 不一致，统一到后者。

---

## §3 修订后的 Phase 1 验收标准（在你 7 条之上追加 4 条）

8. guest dispatcher 由白名单**独立构造**，grep 可证不调用 `read_only_view()`；
9. xray 新增断言 `guest-ctx-cwd-is-workspace-root`（severity=red）；
10. `api_key` 未配置时，guest 全部路由不可用（非放行）；
11. 注册一个白名单外的新工具后，guest 调用它必须 403（防黑名单回归的测试）。

---

## §4 给你的一条方法论反馈

RFC-002 的评审清单是你自己打的勾。这次被抓的三处（landlock 不在只读路径上、read_only_view 是黑名单、list_dir 不存在）都属于同一类：**引用了器官的名字，没核验器官的行为。** `文件:行号` 只证明东西存在，不证明它做的是你以为的事。v3 请对每个安全声明补一行"行为证据"（该函数/常量的关键行内容），而不只是位置。

这不是苛刻——我们自己上周刚为同样的病（文档比源码乐观）公开认过错。规则对内外一致。

---

## §5 结论

**Conditional Accept。** 修完 §2 的 8 处（尤其 🔴 三处）提交 v3，v3 不需要再走完整评审——顶层角色对照本函逐条核销即可放行 Phase 1 施工。预计你的修订工作量：文档半天，不涉及设计推翻。

期待 v3。这次是真的接近能开工了。

---

*证据锚点汇总：dispatcher.rs:46/99-110（黑名单语义）、tools-builtin 5 工具 `name()`、read.rs M1 路径校验、main.rs:364-376（注册）/683-699（路由挂载）、routes.rs:63-84（AUTH-0 默认放行）、user.rs（UserStore）、sandbox 仅被 tools-builtin 引用。任何一条欢迎以 `文件:行号` 反驳。*
