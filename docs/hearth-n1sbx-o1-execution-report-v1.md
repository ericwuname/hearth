# Hearth 窗口执行报告 v1：N1-SBX 施工 + O-1 + P1-LTR-01 设计 + v0.2.10 发版（2026-08-30）

> **依据**：顶层《收到〈W3W4 回归验证 · 新基线复验报告 v1〉》（W3/W4 CLOSED + 三裁决）+ 守门员裁决 1/2/3。  
> **commit 链**：`f8f549a`(N1-SBX) → `1d5bd86`(O-1) → `c8b6532`(fmt) → `fb93f48`(v0.2.10 资产) → **tag `v0.2.10`**。

## 一、裁决 1 · N1-SBX 施工（批准后执行）——✅ 验收通过

- **实施**（sandbox/src/lib.rs，+108 行）：`FILE_MASK`（文件级 4 位全集）/`DIR_ONLY_MASK`（11 位，REFER 归档入内）/`FS_FILE_ONLY = FS_RW & FILE_MASK`/`FS_RO_FILE`（增 1 同修）——动态掩出 + 编译期覆盖性断言（15 位无交叠无遗漏，增 2）；`add_landlock_rule()` fstat 类型感知、调用点零改动（增 3）；fail-closed warn+skip 分毫不动（增 7）。
- **先红后绿**：位运算取证旧集 2 红（FS_RW 含 11 目录专有位 / FS_RO 含 READ_DIR）；4 个数学单测绿。
- **真机两层验收**（顶层明令，缺一不可）：
  1. 规则层 ✅：运行日志**零条** `landlock_add_rule failed ... EINVAL`；
  2. 执行层 ✅：沙箱内 `echo hi >/dev/null && echo TC8_EXEC_OK` 退出码 0、输出 TC8_EXEC_OK——**TC-8 完整转绿**（判定层 RC24 + 执行层 N1-SBX）。
- **不回归 ✅**：可写目录文件写/读（`dir_rule_ok`）、只读路径（`ls /usr/bin`）正常；gate `.133:/home/wutao/t_gate_n1sbx_o1.log`：fmt=0 clippy=0 RT4=0 **411 passed / 0 failed**（+4 N1-SBX 单测）。
- **总账回填**（增 5）：T6 条目勘误留痕（"自 v0.2.3 起从未生效"，f0b4a54 假绿同族）+ TC-8/TC-9 闭环批注。

## 二、裁决 2 · O-1 施工——✅ 验收通过

- **实施**（run_local.rs，+12 行）：budget_reassess/clarification 交互分支 `is_terminal` guard（CLI 消费端，照抄 RC24-B approval 先例）。**落点说明**：守门员锚点 loop.rs:3835-3860 的 guard 经由 CLI 消费端实现——loop 层零改动、内核不碰 IO，与 approval 修复完全同构（实现落点自决，语义一致）。
- **真机验收**：V2R2 同款场景（budget 6 多步任务 headless）→ guard 触发提示 + 秒级 `✗ Task failed — budget_exhausted（6 步）` 结构化收口（**对照 V2R2 卡 200s 无终态**）。

## 三、裁决 3 · P1-LTR-01 设计单——⏸ 停设计待批准

`docs/hearth-p1-ltr-01-tool-deadline-construction-order-v1.md`：三问已答（落点=H1 declared_timeout 通道单点 min / 剩余量=共享截止时刻戳 ToolContext.task_deadline / terminal reason=任务剩余期截断归 `deadline_exceeded` 复用 H2 收尾）+ ChatGPT 六要素覆盖 + 5 项测试矩阵 + 回滚方案。预估 <60 行。**未写代码。**

## 四、版本资产（守门员方案 B）

- 版本 **0.2.9 → 0.2.10** + `CHANGELOG-v0.2.10.md`（RC24/W8/N1-SBX/O-1 四窗口汇总 + OPEN 缺口清单）+ tag `v0.2.10`。
- 总账回填见上。

## 五、版本同步（守门员增 8 · 双重 provenance）

| provenance | .133（评审主战场）                  | .131（执行窗口）                                                     |
| ---------- | ---------------------------- | -------------------------------------------------------------- |
| source     | 900d199+（门禁 411）             | git archive HEAD → `~/codex`（Cargo 0.2.7→**0.2.10**，target 保留） |
| binary     | release 0.2.10（411 gate 后构建） | release 重编译 **`hearth 0.2.10`** 实证 ✅                           |

## 六、新发现（observe-only，本单禁动）

- **O-3**：seccomp allowlist 缺 `SYS_MKDIRAT(258)`——`mkdir` 等建目录命令 SIGSYS（exit 159）。与总账已录 chmod/getent 159 同族（fail-closed 方向，安全无损功能受限）；**N1-SBX 清白**（diff 全在 landlock 权限集数值，seccomp BPF 分毫未动；SIGSYS 只能源于 seccomp）。G0 变更须独立批准——目录规则验证已改用无 mkdir 方式完成（可写目录内文件写/读）。
- Q-3（守门员已立）：landlock ABI≥5 未 handled 位——本单未触碰。

## 七、状态总表

```text
CLOSED: R2-C / W3W4 / RC24 / W8 / N-2 / N1-SBX / O-1
DESIGN: P1-LTR-01（待批准）
OPEN:   DEV-2（独立研究）/ OPEN-W8-1（Task Type+Constraints 分离）/ verify_failed fixture / N-3 / O-3 / Q-3
DEFER:  Bridge / Subagent / TUI / MCP
VERSION: v0.2.10 tagged（双 VM 对齐）
```

**本轮停止，等顶层验收 N1-SBX/O-1 执行 + P1-LTR-01 设计批准。**

