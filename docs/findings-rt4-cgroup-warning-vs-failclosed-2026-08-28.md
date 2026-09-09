# 发现：RT4 启动警告与沙箱实际行为不一致（会影响 T7 遥测有效性）

> 类别：安全不变量 / 运维语义正确性　｜　发现窗口：评审（守门人）　｜　日期：2026-08-28
> 性质：**只读源码取证**，未改动任何实现代码。
> 紧急度：中（不阻塞 T1-T6，但 **T7 开始前必须处置**，否则 T7 数据被环境阻塞污染）

---

## 1. 事实链（全部有源码锚点）

| # | 事实 | 源码锚点 |
|---|---|---|
| 1 | 每次工具 spawn **无条件**调用 cgroup 应用（无限额配置守卫，且带 `?` 传播错误） | `crates/sandbox/src/lib.rs:1035` `apply_cgroups_impl(pid, &cfg_for_cgroups_inner)?;` |
| 2 | cgroup 不可用 + 未开降级 → **Err「安全限制无法保证（RT4 fail-closed）」**，工具被硬拦 | `crates/sandbox/src/lib.rs:839-852` |
| 3 | 降级开关 `HEARTH_ALLOW_NO_CGROUP` **全仓仅被读取**（`lib.rs:828`），**无任何代码 `set_var`** —— 只能由用户手动设 env | 全仓 grep `HEARTH_ALLOW_NO_CGROUP`：13 处命中全为读取/文案，无写入 |
| 4 | 启动警告 `cgroup_warning()` 是**纯探测函数，只返回字符串**，不设任何 env | `crates/codex-cli/src/run_local.rs:35-54` |
| 5 | 警告文案：「…**资源限制不会生效**。设 HEARTH_CGROUP_BASE=<delegation 子树> 或 HEARTH_ALLOW_NO_CGROUP=1 显式降级」 | `crates/codex-cli/src/run_local.rs:41` / `:51` |
| 6 | v0.1.1 该改动动机注释：「cgroup 不可用…时**工具全被 fail-closed 拦**——**启动即警告**」 | `crates/codex-cli/src/run_local.rs:270-271` |

---

## 2. 问题：警告描述与实际后果不符

- **警告暗示**：工具会运行，只是没有资源上限（"degraded but working"）。
- **实际行为**：cgroup 不可用时工具**根本无法运行**（fail-closed 硬拦），后果比"没有上限"严重得多。
- 且按锚点 6 的注释，v0.1.1 本意是修「工具全被 fail-closed 拦」，但**只增加了启动警告，并未真正解除阻塞**（锚点 3 证明无人自动设降级开关）。

**定性**：这是**运维语义错误**（误导排障方向）+ **一个可能未完成的修复**（v0.1.1 的意图 vs 实际落地），**不是**新的安全漏洞——fail-closed 本身是正确的安全姿态。

---

## 3. 对执行窗口的直接影响（时间敏感）

**T7（长任务 + 真实工具组合场景遥测）会撞上这条**：
- T7 首次大规模引入真实工具调用；
- 在**无 cgroup delegation 的 VM**（如当前测试环境）上，且不设 `HEARTH_ALLOW_NO_CGROUP=1` 时，**每一次工具 spawn 都会被 fail-closed 拦下**；
- 结果：T7 测出的"失败"是**环境阻塞**，而非要测的可靠性回归 → **T7 基线被污染，无法与纯推理场景对比**。

**处置选项（待顶层拍板，评审不下处方）**：

| 选项 | 后果 |
|---|---|
| (a) T7 运行时显式设 `HEARTH_ALLOW_NO_CGROUP=1` | 工具可跑、无资源上限；T7 数据可比，但**测得的是"无 cgroup 限制"这一档**的真实表现，须在报告注明 |
| (b) 为 VM 配 cgroup delegation 子树并设 `HEARTH_CGROUP_BASE` | 最接近生产形态，但需 VM 侧配置（非特权 userns 受限，可行性待验） |
| (c) 先修文案/行为不一致（让警告与实际一致），再跑 T7 | 彻底但会阻塞 T7 排期 |

**无论选哪个，T7 开始都必须先明确记录"跑 T7 时 cgroup 处于哪一档"，否则数据不可解释。**

---

## 4. 附带确认（好消息）

评审早先 handoff（`docs/handoff-claude-r2-resolution-2026-08-28.md`）的 RT4 patch **已被执行窗口正确落地**：
- `SandboxConfig` 已含 `cgroup_base_override: Option<PathBuf>`（`sandbox/lib.rs:63-68`，注释「R2 修复（handoff §6 Patch A）」），`Default`/`for_build_tools` 均置 `None`（`lib.rs:92`/`124`）；
- `apply_cgroups_impl` base 解析采用 override > env > 默认（`lib.rs:833-837`）；
- `cleanup_cgroup(pid, cfg: &SandboxConfig)` 签名已同步（`lib.rs:934`）；
- **关键**：`test_rt4_cgroup_fail_closed` 已改为 `panic!` 强制态（`lib.rs:1694-1696`：「spawn 未触发 fail-closed —— 本测试**必须不**设置 HEARTH_ALLOW_NO_CGROUP=1…」），即 Claude 要求的"禁止静默 SKIP"已落实。

→ 待执行窗口在 VM 实跑该测试后，证据包 §A 的权威计数应由 `282/0/1` 更新为 `283/0/0`。

---

## 5. 覆盖边界（诚实标注）

- 本发现基于**源码静态取证**，未在 VM 上实测"工具 spawn 被拦"的完整错误输出（执行窗口可在 T7 前用一次真实工具调用验证，成本极低）。
- 未评估：若 (c) 选项成立，改文案 vs 改行为哪个更符合产品意图——属顶层判断。
