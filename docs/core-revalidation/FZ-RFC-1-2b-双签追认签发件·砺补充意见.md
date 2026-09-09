# FZ-RFC-1 / 2b 双签追认签发件 · 砺补充意见

> **评审**：砺·评审（🪨）　**日期**：2026-09-03　**对象**：`docs/core-revalidation/FZ-RFC-1-2b-双签追认签发件.md`
> **结论**：**RFC-2b 证据充分，可签**；**RFC-1 建议补 1 项测试 + 修正 3 处口径后再签**（均为低成本）。
> **纪律**：本件所有锚点为砺亲自 grep+read 实测；行号漂移已实证（见 §5.3）。

---

## 0. 结论速览

| 项 | 判定 | 阻塞签名？ |
|---|---|---|
| RFC-2b 全部锚点 | ✅ 与申报一致 | 否 |
| RFC-1 白名单数字 | ❌ 三处口径互不一致 | **是（文字修正）** |
| RFC-1 实施证据强度 | ⚠️ 仅单元测试，未覆盖 spawn 路径 | **建议补测** |
| RFC-1 不变量表述 | ⚠️ dev/prod 语义不等价 | 建议修正 |
| 兼容性行为变更 | ⚠️ 未登记 | 建议补登记 |
| 回退方案 | ❌ 缺失 | 建议补 |
| 外部 AI 窗核验清单 | ❌ 缺失 | 建议补（§6 已备） |

---

## 1. RFC-2b：证据充分 ✅（可签）

| 申报项 | 源码实测 |
|---|---|
| `rc52_route_done_if_session_artifacts()` | `loop.rs:2116` ✅ |
| `completion_fact_check()` 前移 | `loop.rs:4105` ✅ |
| budget 臂锁存 `fa01_budget_intercepted` | 字段 `:1023` / 初始化 `:1548` / 重置 `:4779` / 判断 `:4855` / 锁存 `:4889`、`:4918`（`:4914` 注释明载"必须先置锁存再 continue——否则"防重入）✅ |
| RED 集成测试 `test_rc52_hydration_integration_through_run_boundary` | `loop.rs:9888` ✅ |

**残留登记（B 臂 = OD-1 🟡）与本窗 open-deviations 评审一致，无异议。**

---

## 2. RFC-1 · 白名单数字三处不一致（**必须修正**）

| 出处 | 声称 | 实测 |
|---|---|---|
| 申报栏 | "PATH/HOME/LANG/LC_ALL/TMPDIR" = **5 项** | — |
| 实施证据栏 | "白名单 **6 项**精确性" | 源码中**无 6 项对应物** |
| 源码 `minimal_child_env()` | — | **4 项**：`["HOME","LANG","LC_ALL","TMPDIR"]`（`sandbox/src/lib.rs:2127`） |

**根因**：PATH **不在**白名单函数内，而是由 **bash.rs:302-321 在工具层显式注入**（`env.push(("PATH", ...))`，全仓仅此一处）。签发件把"沙箱层白名单"与"工具层显式注入"两个机制合并表述，导致计数失真。

**建议修正表述**：
> 沙箱层最小白名单 = 4 项（HOME/LANG/LC_ALL/TMPDIR，`lib.rs:2127`）；PATH 由 bash 工具层显式注入（`bash.rs:302-321`），经 `.envs(env_vars)` 后置覆盖；二者合计 5 项生效。

---

## 3. RFC-1 · 实施证据强度不足（**建议补测**）

**现有两个 s4 测试均为 `minimal_child_env()` 的纯单元测试**：

| 测试 | 位置 | 断言内容 |
|---|---|---|
| `test_minimal_child_env_whitelist_only` | `lib.rs:2138` | 函数返回值中：marker 不在 + HOME 在 |
| `test_minimal_child_env_no_secrets_by_default` | `lib.rs:2157` | 函数返回值中：不含 "AGNES" |

**三点未覆盖**：

1. **未验证 spawn 路径真的执行了 env_clear**——测试只调函数，未起子进程；
2. **未验证子进程实际环境**（无 `/proc/<pid>/environ` 或执行 `env` 命令级断言）；
3. **未验证"显式注入后置覆盖"这一关键机制**（`lib.rs:1156-1158` 的组合 `.env_clear() → .envs(minimal) → .envs(env_vars)`）。

**第 3 点是真实风险**：若组合顺序或内容有误，`tool_env` 注入的 `HEARTH_EGRESS_ALLOWLIST` / `HEARTH_READ_ROOTS` 会被清掉 → **egress 白名单与读根限域静默失效**（与本 RFC 意图相反的反向安全回归）。而现有测试对此完全不设防。

**建议补一个 spawn 级集成测试**（低成本，1 个用例）：

```rust
// 断言子进程实际环境（sandbox/tests 或 lib.rs tests）
// 起子进程执行 `env`，断言：
//   ① HOME/LANG/LC_ALL/TMPDIR 存在（白名单 4 项）
//   ② 父进程注入的 HEARTH_S4_SECRET_MARKER 不可见
//   ③ 显式注入的 HEARTH_EGRESS_ALLOWLIST / HEARTH_READ_ROOTS 仍在（后置覆盖生效）
//   ④ PATH 在 bash 工具路径下存在
```

> 补测后 gate 复跑即可，不触冻结语义。**建议作为签署前置（低成本高价值）。**

---

## 4. RFC-1 · 不变量表述不精确（建议修正）

签发件称"dev `NoopSandbox:185` env_clear **语义对齐**"。实测**不等价**：

| 侧 | 实现 | 位置 |
|---|---|---|
| dev `NoopSandbox` | `env_clear()` + **仅调用方传入 env**（**无白名单**） | `lib.rs:185-186` |
| prod `LinuxSandbox` | `env_clear()` + **minimal_child_env()** + 调用方 env | `lib.rs:1156-1158` |

dev 侧**没有白名单兜底**（传入空 env 则子进程环境全空）；prod 侧有 4 项兜底。二者是**部分对齐**，非对齐。

**建议修正**：改为"dev 侧已有 env_clear（早于生产侧，倒挂），本 RFC 补齐生产侧并增加最小白名单兜底；两侧清空语义一致，白名单兜底仅生产侧有"。

---

## 5. 未登记项（建议补入）

### 5.1 白名单外变量的兼容性影响

`env_clear` 后，白名单外一切变量对**工具子进程**不可见，包括常见开发环境变量：`CARGO_HOME` / `RUSTUP_HOME` / `GOPATH` / `http_proxy` / `https_proxy` / `no_proxy` 等。

- gate 486/0 全绿 → 现有测试链路不依赖；
- 但**沙箱内 `cargo test` 等场景若依赖 CARGO_HOME/RUSTUP_HOME，行为会变**。
- **建议**：显式登记为已知行为变更，并确认 campaign / 真机测试链路不依赖这些变量（尤其 `.131`/`.133` 上的 `HEARTH_ALLOW_NO_CGROUP`、`HEARTH_CGROUP_BASE`——注意这两者由 **hearth 父进程**读取，不经子进程，故不受影响；但应在件中写明以免后续误判）。

### 5.2 非 bash 工具无 PATH

仅 bash 工具注入 PATH（`bash.rs:318`）。其余经沙箱起子进程的工具不带 PATH，例：
- `has_rg()`（`grep.rs:169-170`）以 **空 env `&[]`** 调 `sandbox.spawn("rg", ...)` → 子进程无 PATH → `rg` 查找失败 → 回退非 rg 分支。

gate 全绿且 dev 侧同行为，故**非回归、非阻塞**；但属**用户可感行为**，冻结材料应显式登记，免得 v0.3 排障时重复发现。

### 5.3 全篇无 file:line 锚点 + 行号漂移实证

签发件全篇未给行号（RFC-2b 侧有函数名，尚可），外部 AI 窗核验须重新 grep。**行号漂移已实证**：`rc52_route_done_if_session_artifacts` 在本窗记忆中为 `:2110`，实测现为 `:2116`（**+6**）。

**建议**：签发件附锚点时标注"核验当日实测"，避免外部窗按旧记录核对后误判"锚点不符"。

---

## 6. 补：外部 AI 窗最小交叉核验清单（可直接附入签发件）

外部签位"可交叉核对本件所列证据与源码"——建议附以下清单（**核验前须重新 grep，勿用记忆中的行号**）：

**RFC-1（4 查）**
1. `grep -n "minimal_child_env" crates/sandbox/src/lib.rs` → 白名单是否 4 项（HOME/LANG/LC_ALL/TMPDIR）
2. `grep -n "env_clear" crates/sandbox/src/lib.rs` → 是否 2 处（`:185` dev、`:1156` prod）
3. `grep -n "PATH" crates/tools-builtin/src/bash.rs` → PATH 是否仅在工具层注入
4. `cargo test -p sandbox s4` → 2 个 s4 测试是否通过；**并确认二者均为单元测试**（未覆盖 spawn）

**RFC-2b（4 查）**
5. `grep -n "fn rc52_route_done_if_session_artifacts" crates/agent-core/src/loop.rs`
6. `grep -n "fn completion_fact_check" crates/agent-core/src/loop.rs`
7. `grep -n "fa01_budget_intercepted" crates/agent-core/src/loop.rs` → 锁存是否 2 处（`:4889`、`:4918`）
8. `grep -n "test_rc52_hydration_integration_through_run_boundary" crates/agent-core/src/loop.rs` → 该测试是否存在且通过

**门禁**：`cargo test`（`unset HEARTH_URL` + `HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth`，**不设** `HEARTH_ALLOW_NO_CGROUP`）→ 期望 486/0。

---

## 7. 建议签署条件

| # | 条件 | 责任方 | 阻塞 |
|---|---|---|---|
| S-1 | 修正白名单数字（5/6/4 → 明确"4 项白名单 + 1 项工具层 PATH"） | 执行窗 | 是（文字） |
| S-2 | 补 spawn 级集成测试（§3 四项断言） | 执行窗 | **建议是** |
| S-3 | 修正 dev/prod"语义对齐"表述 | 执行窗 | 否 |
| S-4 | 登记兼容性影响（§5.1/§5.2） | 执行窗 | 否 |
| S-5 | 附回退方案（否决时改哪几处、如何验证等效） | 执行窗 | 否 |
| S-6 | 附 §6 核验清单供外部 AI 窗使用 | 执行窗（可直引本件） | 否 |

**砺意见**：S-1 修正 + S-6 附上后，RFC-2b 可直接签；**RFC-1 建议 S-2 补测后签**（成本低，且补的是"防反向安全回归"这一关键缺口）。

---

> **砺·评审（🪨）**　2026-09-03
> 全程只读；所有锚点均为砺亲自 grep+read 实测，未采信任何转述。
