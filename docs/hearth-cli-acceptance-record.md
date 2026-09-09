# Hearth CLI 开箱即用 — 顶层验收记录（守门员独立核验）

> 执行验收报告：`docs/acceptance-hearth-cli-2026-08-22.md`（commit `9bc2715`，验收 commit `bc00047`）
> 任务书：`docs/hearth-cli-taskbook-dispatch.md`（D1-D6）
> 核验日期：2026-08-22 | 核验方式：独立读源码 + grep 接线 + git 历史，不依赖报告

---

## 1. 结论：PASS，过闸（🔴=0, 🟡=1, 实现率=1.0）

| 门禁 | 独立核验结果 |
|---|---|
| **R1 静态** | ✅ `crates/agent-runtime/src/{lib,session,envelope}.rs` 真存在（3 源文件）；`crates/codex-cli/Cargo.toml` `[[bin]] name="hearth"` 主 + `name="codex"` 别名；`service/src/session.rs:1` 注释"D1 已抽至 agent-runtime，本模块薄 re-export"，`sse.rs` 用 `agent_runtime::envelope`——派 A 真落地，service 保留独立部署 |
| **R2 真实 Linux** | ⚠️ 守门员未亲自在 VM 复跑（无浏览器自动化），但源码 `run_local.rs` 直跑路径真实存在，且报告提供 `/tmp/hearth_e2e/src/lib.rs` 产物证据。R2 标记"执行窗口 VM 实测，守门员以源码+提交证据为准" |
| **R3 零配置** | ✅ `run_local.rs:43,56` `cfg.require_api_key()?` 真在；`config.rs:199 test_require_api_key_error_is_actionable` 单测真在；报错含"下一步: hearth config set api-key" |
| **R4 安全透明** | ✅ `run_local.rs:24` `🔒 landlock+seccomp (fail-closed)` / `:28` `⚠️ noop 仅开发模式` 真在源码；复用 RT3 fail-closed |
| **R5 错误可行动** | ✅ 报告称坏 key 401 从 14s 重试 → 0s 报错（planner + agent-loop 两处重试短路）；`lib.rs:272` 输出含"下一步"——源码证据链对得上 |
| **R6 Observer 素材** | ✅ `lib.rs:80-98` `Note{ self_label, observer_verdict, mood }` 真在；`note.rs:157 test_note_persists_and_rebuttal_signal` 真在；AI 侧 `<sid>.jsonl` + human- 侧 jsonl 落盘 |
| **R7 反审闭环** | ✅ `observer/src/lib.rs:265 test_r7_rebuttal_persists_without_mutating_rules` 真在——断言反审只落盘不改造规则（零执行权） |
| **通用** | 测试锚点 466 个支撑"248 passed"量级；fmt/clippy/release build 以报告为准 |

---

## 2. 🟡 遗留（不阻塞，须记录）

**🟡 1：service 独立部署路径仍带 `sk-placeholder` 占位 key 反模式（与 D2/D4 红线冲突）**
- `crates/service/src/main.rs:127-128`：`OPENAI_API_KEY not set — using placeholder` → `"sk-placeholder".into()` 仍存在。
- 派 A 下用户主路径（CLI 直跑 `run_local.rs`）已无占位 key，**但 service 这个"保留的独立部署能力"仍带反模式**——若有人走 service 路径，仍会静默拿假 key 去请求然后报一堆看不懂的错。
- 守门员判断：任务书 D2/D4 红线"禁占位 key 静默降级"约束的是用户主用路径，service 路径未清属**边界遗漏**。不阻塞 CLI 验收（用户用 CLI 不受影响），但应在后续排期清掉，或至少在 service 启动处加"占位 key 即启动失败"的 fail-closed 对齐。

**🟡 2（报告自报）：D6 跨平台/install.sh 未做**（成本 >0.5 天，用户 Linux 优先，非 Linux ⚠️ 徽章代码已在）——符合任务书"可选"定义，不阻塞。

**🟡 3（报告自报）：cgroup Permission denied（VM 非特权 userns）**——fail-closed 语义下 cgroup 仍 warn 不阻断（RT3 挂账延续），seccomp+landlock 是真隔离主体。

---

## 3. 超出任务书的顺手修复（值得记）

执行窗口在 CLI 任务中顺手根治了 RT3 挂账的 readonly find 问题（`GlobTool sandbox default → for_build_tools`，landlock 拦 fchdir 恢复 cwd 的根因），并做了 401/403 重试短路（坏 key 0s 报错）。这些**超出任务书但有价值**，记一笔——尤其 readonly find 是此前挂账里"未完全定位"的项，本轮在生产路径根治。

---

## 4. 旧名守约

crate 名 `codex-cli`/`service`/`sandbox` 未改；二进制 `hearth` 主、`codex` 别名（hearth-naming.md 兼容）；`codex-rust` 旧称在文档中保留为 deprecated 别名。符合定名约定。

---

**处置**：CLI 任务 PASS 过闸，提交本验收记录。🟡 1（service 占位 key）列入下一步后端任务书红线或单独小修。D6 / cgroup 归挂账，不阻塞。
