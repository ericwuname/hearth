# Hearth N1-SBX 施工单 v1：Landlock File Rights 修复（设计稿 · 待批准）

> **性质**：独立施工单设计稿——**批准后施工，本文档不含代码变更**。
> **签发依据**：顶层《收到〈Hearth W8 验收报告 v1〉及 N-1 诊断报告》——N-1 正式进入独立 G0 Sandbox 修复 + 守门员补充 8 条（2026-08-29 23:55，与正文同效力）。
> **诊断依据**：`docs/n1-landlock-devnull-diagnosis.md`（C 矩阵实测，kernel 7.0.0-30）+ landlock_add_rule(2) man page 原文。
> **范围红线**：只动 `crates/sandbox/src/lib.rs` 的权限集常量与 `add_landlock_rule()` 内部；**禁止触碰** ContextBuilder / TaskGraph / Goal Revision / Reflect decision / ApprovalPolicy / RC24 审批语义（审批判定层零改动——TC-8/TC-9 判定表已在 RC24 定格）。

---

## 条款 1 · 当前 FS_RW / FS_RO 真实 rights 集（实测锚点 `crates/sandbox/src/lib.rs:234-267`）

Landlock FS 权限位全 15 个（ABI v1-v5）：

| 位 | 常量 | 值 | 适用对象 |
|---|---|---|---|
| 0 | EXECUTE | 1<<0 | 文件+目录 |
| 1 | WRITE_FILE | 1<<1 | 仅文件 |
| 2 | READ_FILE | 1<<2 | 仅文件 |
| 3 | **READ_DIR** | 1<<3 | **仅目录** |
| 4 | **REMOVE_DIR** | 1<<4 | **仅目录** |
| 5 | **REMOVE_FILE** | 1<<5 | **仅目录**（unlink 作用于父目录） |
| 6 | **MAKE_CHAR** | 1<<6 | **仅目录** |
| 7 | **MAKE_DIR** | 1<<7 | **仅目录** |
| 8 | **MAKE_REG** | 1<<8 | **仅目录** |
| 9 | **MAKE_SOCK** | 1<<9 | **仅目录** |
| 10 | **MAKE_FIFO** | 1<<10 | **仅目录** |
| 11 | **MAKE_BLOCK** | 1<<11 | **仅目录** |
| 12 | **MAKE_SYM** | 1<<12 | **仅目录** |
| 13 | **REFER** | 1<<13 | **仅目录**（跨层次链接/重命名） |
| 14 | TRUNCATE | 1<<14 | 仅文件 |

当前复合集（**病根标注**）：

```text
FS_RO = READ_FILE | READ_DIR | EXECUTE                      // :251 —— READ_DIR 是目录专有位
FS_RW = FS_RO | WRITE_FILE | REMOVE_DIR | REMOVE_FILE
        | MAKE_CHAR | MAKE_DIR | MAKE_REG | MAKE_SOCK
        | MAKE_FIFO | MAKE_BLOCK | MAKE_SYM | REFER
        | TRUNCATE                                           // :255-267 —— 11 个目录专有位
```

当前调用点（三处，全部传全量集）：
- `:377` writable_paths 循环 → `add_landlock_rule(path, FS_RW)`
- `:382` read_only_paths 循环 → `add_landlock_rule(path, FS_RO)`
- `:391` T6 `/dev/null` 显式放行 → `add_landlock_rule("/dev/null", FS_RW)` —— **必 EINVAL**（目标 = char device ≠ 目录）

**同族隐患（守门员增 1）**：FS_RO 同病——`READ_DIR` 混入。当前 read_only_paths 全为目录无实际炸点，但机制上任何普通文件目标静默 EINVAL（warn+skip）。**本单必须同时修 FS_RW 与 FS_RO 两个侧**。

## 条款 2 · Landlock object type / rights 适用关系（内核依据）

- **内核规则**（landlock_add_rule(2) ERRORS 原文）："some access rights in rule_attr->allowed_access are **only applicable to directories**, but rule_attr->parent_fd does not refer to a directory" → **EINVAL**。
- **DIR_ONLY_MASK**（11 位）：READ_DIR | REMOVE_DIR | REMOVE_FILE | MAKE_CHAR | MAKE_DIR | MAKE_REG | MAKE_SOCK | MAKE_FIFO | MAKE_BLOCK | MAKE_SYM | REFER
  —— **REFER 必须列入**（守门员增 2 点名：诊断报告 FS_FILE_ONLY 恰好不含它，但未点名；本单把 REFER 正式归档为 DIR_ONLY，防未来权限集演进时误留）。
- **FILE_MASK**（4 位，恰为内核"适用于文件"全集）：EXECUTE | WRITE_FILE | READ_FILE | TRUNCATE
- **实证矩阵**（`docs/data/n1-landlock/ll_diag.c`，已归档，kernel 7.0.0-30）：

| 用例 | parent_fd | 权限集 | 结果 |
|---|---|---|---|
| C1 | /dev/null（char dev） | FS_RW 全量 | EINVAL |
| C2 | /dev/null（char dev） | 文件级四位 | **OK** |
| C3 | 常规文件 | FS_RW 全量 | EINVAL（非 char-device 特例的决定性证据） |
| C3b | 常规文件 | 文件级四位 | OK |
| C4 | 目录 | FS_RW 全量 | OK |
| C5 | /dev/full（char dev） | 文件级四位 | OK |

## 条款 3 · 权限集定义（动态掩出，禁止手写常量集——守门员增 2）

```text
FILE_MASK     = EXECUTE | WRITE_FILE | READ_FILE | TRUNCATE        // 文件级四位全集
DIR_ONLY_MASK = READ_DIR | REMOVE_DIR | REMOVE_FILE | MAKE_CHAR
              | MAKE_DIR | MAKE_REG | MAKE_SOCK | MAKE_FIFO
              | MAKE_BLOCK | MAKE_SYM | REFER                      // 11 位目录专有全集

FS_FILE_ONLY = FS_RW & FILE_MASK      // 动态掩出（const 数学）
FS_RO_FILE   = FS_RO & FILE_MASK      // = READ_FILE | EXECUTE（增 1：FS_RO 侧同修）
```

- **禁止手写四常量集**——未来 ABI v6+ 新位进入 FS_RW/FS_RO 时 `& FILE_MASK` 自动正确；手写集会静默漏位。
- **静态断言**（编译期或单测）：`FS_FILE_ONLY & DIR_ONLY_MASK == 0`、`FS_RO_FILE & DIR_ONLY_MASK == 0`、`DIR_ONLY_MASK | FILE_MASK == 全 15 位`（无重叠无遗漏——覆盖性断言防枚举漂移）。
- 既有 `FS_RO` / `FS_RW` 常量**保持不变**（目录路径仍用全量集——语义无收窄）。

## 条款 4 · `add_landlock_rule()` 目标类型处理（fstat 感知，调用点零改动——守门员增 3）

- 实现：`add_landlock_rule()` 内部 `File::open` 后对 `parent_fd` 做 `fstat` → `S_ISDIR(st_mode)`：
  - 目录 → 传 `FS_RO`/`FS_RW`（现状全量集）
  - 非目录 → 传 `FS_RO_FILE`/`FS_FILE_ONLY`（文件级掩出集）
- **调用点 `:377/:382/:391` 一行不改**——类型感知收敛在单点，比"每个调用点记得选对集合"稳健；顺带根治 writable_paths 未来含常规文件的静默 EINVAL 同族隐患；`/dev/null` 无需特判。
- open 方式（`File::open`，非 O_PATH）保持不变——O_PATH 优化不纳入本单（最小 diff）。

## 条款 5 · 测试矩阵（/dev/null · 普通文件 · 目录）

| # | 场景 | 断言 |
|---|---|---|
| T-1 | `/dev/null` 规则添加（T6 路径） | 无 EINVAL warn；`landlock_restrict_self` 后 `echo x > /dev/null` 沙箱内成功 |
| T-2 | 普通文件目标（writable/read-only 各一） | 规则添加成功，文件读/写按语义放行 |
| T-3 | 目录目标（writable/read-only 各一） | **不回归**——全量集照常生效（目录内创建/删除正常） |
| T-4 | 权限集数学 | FS_FILE_ONLY/FS_RO_FILE & DIR_ONLY_MASK == 0；DIR_ONLY∪FILE == 全 15 位 |
| T-5 | TC-8 场景重跑 | `echo hi >/dev/null` headless 全链路成功（判定层已绿 + 执行层本单转绿） |

## 条款 6 · 负面测试（锁定非法组合）

- **本地可跑（平台无关，先红后绿——守门员增 4）**：权限集数学断言（T-4）——修复前旧 FS_RW/FS_RO 无掩出，数学断言红；修复后绿。Windows 本机即可跑（纯常量数学，不依赖内核）。
- **VM 真机**：ll_diag 矩阵重跑——C1（FS_RW 全量 + char dev）**必须保持 EINVAL**（这是内核语义锚，本单不修内核、只修我们传的参数）；C2/C5 保持 OK。内核语义锚变红 = 内核行为变化 = 偏差报告停点。

## 条款 7 · 真机 kernel 7.0.0-30 验证（两层，缺一不可——顶层明令）

1. **规则层**：`landlock_add_rule()` 成功——运行日志零条 `landlock_add_rule failed ... EINVAL`（对照 RC24 验收①的 WARN 复现）。
2. **执行层**：实际 `/dev/null` 写入成功——沙箱内 bash `echo rc24-tc8-ok >/dev/null && echo OK` 退出码 0（对照 RC24 验收①的 `权限不够` 失败）。
3. **不回归**：普通文件规则（T-2）、目录规则（T-3）、既有 bash 全量行为（构建/测试类命令）正常。

## 条款 8 · fail-closed 不变量（守门员增 7，显式声明）

- **本单只是让 add 成功；add 失败时的 warn+skip 行为分毫不动**——skip 后 `landlock_restrict_self` 依旧全拒 = fail-closed 语义保持。
- **禁止**执行窗口顺手"加固"为硬失败（add 失败 → Err）——那是行为变更，需另行批准。
- G0 语义清单（不变量）：landlock 仍 fail-closed 收紧子进程；seccomp allowlist 不动；cgroup/RT4 逻辑不动。

## 条款 9 · 不影响 RC24

- 审批判定层（`bash_cmd_is_destructive` / `classify_bash_cmd` / `ApprovalPolicy`）零触碰。
- InteractionRequested 事件契约零改动。
- TC-8/TC-9 判定表测试（RC24 落地）原样通过——本单只补执行层。

## 条款 10 · 回滚方案

- 单 commit（sandbox crate 单文件 diff），可 `git revert` 一键回滚。
- 回滚后状态 = 现状（/dev/null 规则 EINVAL skip → 沙箱内不可写）——**无安全性损失**（现状即 fail-closed），仅功能回退。
- 风险评估：非目录目标权限集从"全量（实际 EINVAL 全拒）"变为"文件级四位（实际放行文件四操作）"——**权限不放大**：landlock 规则只作用于该目标路径自身，且四位均为该文件类型的合法操作；原状态是"规则不存在 + 全拒"，新状态是"规则存在 + 按位放行"，均为 landlock 设计语义内的合法配置。

---

## 验收后事项（不阻塞本单施工，完工时顺路）

1. **总账回填两处**（守门员增 5）：① TC-8/TC-9 完整转绿后回填 `docs/consolidated-remediation-ledger.md`；② **T6 条目勘误**——历史"T6 已落地/已验收"改写为"T6 自 v0.2.3 起从未生效（add_rule EINVAL 静默 skip），被旧审批门遮挡至 RC24 才暴露"（与 f0b4a54 假绿同族，必须留痕）。
2. **.131 源码树同步**（vm-version-sync，守门员增 8）——N1-SBX 合入后一次做掉，避免孤儿状态（.131 当前停在 0.2.9+W3/W4）。
3. **门禁日志登记约定执行**（守门员增 6）：施工报告"门禁"节写明每份 gate log 的完整 VM 路径（host + path），批次名入文件名。

## 验收门禁

- 隔离门禁 `~/run_gate_r2c.sh`（.133）全绿（≥407，+本单新增单测）；**gate log 完整路径写入报告**（增 6）。
- 本地权限集数学单测（平台无关）先红后绿证据。
- VM 真机两层验证（条款 7）+ ll_diag 矩阵复跑（条款 6）。
- **gate log 路径登记**（示例格式）：`.133:/home/wutao/t_gate_n1sbx.log`。

## 预估 diff 范围

`crates/sandbox/src/lib.rs` 单文件：+FILE_MASK/DIR_ONLY_MASK/FS_FILE_ONLY/FS_RO_FILE 常量（const 数学）+ `add_landlock_rule` fstat 分支 + 单测（数学断言 + 权限集覆盖性）。**预计 <80 行**。
