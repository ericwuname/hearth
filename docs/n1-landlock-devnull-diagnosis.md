# N-1 诊断报告：landlock 对 /dev/null EINVAL（T-C，只查不改）

> **性质**：诊断单交付（W8 施工单 §三）。**未改动任何代码**。
> 现象：`landlock_add_rule failed for '/dev/null': Invalid argument (os error 22). Rule skipped.`（RC24 验收①首次暴露——被旧审批门遮挡至今）。
> 环境：.133 / kernel `7.0.0-30-generic` / landlock ABI available。

## 一、结论（先行）

**施工单首选假设被实测推翻**。EINVAL 的根因**不是** "/dev/null 是 char device 不被 PATH_BENEATH 支持"，而是：

> **Hearth sandbox 的 `FS_RW` 权限集混入了目录专有权限位（`READ_DIR`/`REMOVE_DIR`/`MAKE_DIR` 等），内核规定"目录专有权限 + 非目录 parent_fd → EINVAL"。任何非目录（包括普通文件）配 FS_RW 全量都会 EINVAL——/dev/null 只是受害者之一。**

内核文档依据（landlock_add_rule(2) ERRORS 原文）：

> EINVAL In struct landlock_path_beneath_attr, the rule accesses are not applicable to the file (i.e., some access rights in rule_attr->allowed_access are **only applicable to directories**, but rule_attr->parent_fd does not refer to a directory).

## 二、实测证据（C 测试矩阵，/tmp/ll_diag.c，kernel 7.0.0-30）

| 用例 | parent_fd 类型 | allowed_access | 结果 |
|---|---|---|---|
| C1 | /dev/null（char dev） | FS_RW 全量 | **errno=22 EINVAL**（真机现象复现） |
| **C2** | /dev/null（char dev） | 纯文件权限（EXEC\|WRITE_FILE\|READ_FILE\|TRUNCATE） | **OK ✅** |
| **C3** | 普通文件 | FS_RW 全量 | **errno=22 EINVAL**（推翻"char device 特例"假设的决定性证据） |
| C3b | 普通文件 | 纯文件权限 | OK |
| C4 | /tmp（目录） | FS_RW 全量 | OK |
| C5 | /dev/full（char dev） | 纯文件权限 | **OK**（char device 完全可作为规则目标） |

## 三、源码定位（crates/sandbox/src/lib.rs，只读）

- `FS_RW = FS_RO | WRITE_FILE | REMOVE_DIR | REMOVE_FILE | MAKE_CHAR | MAKE_DIR | MAKE_REG | ...`（:255 一带）——含目录专有位。
- `FS_RO = READ_FILE | READ_DIR | EXECUTE`（:251）——同样含 `READ_DIR` 目录专有位。
- T6 放行点：`add_landlock_rule(ruleset_fd, "/dev/null", FS_RW)`（:385-393）——**必 EINVAL**。
- `add_landlock_rule` 失败路径 = warn + skip（fail-open for this rule，:807-814）——规则没加上，后续 landlock_restrict_self 依然 enforce 全拒 → `/dev/null` 写 = EPERM。
- **T6 自 v0.2.3 落地以来从未生效**（每条 bash 都在 warn，被日志噪音淹没；此前所有 `>/dev/null` 场景死在审批门，走不到沙箱，故未暴露）。

## 四、修复菜单（修订版，按可行性排序——未经批准不施工）

**菜单 0（新发现，推荐）**：T6 权限集修正——对非目录路径的 landlock 规则改用纯文件权限集（新增 `FS_FILE_ONLY = EXECUTE | WRITE_FILE | READ_FILE | TRUNCATE`），`/dev/null` 用它放行。**一行级改动 + 单测**，T6 真正生效，C2/C5 已证明内核支持。附带修复同族隐患：writable_paths 若含常规文件（当前全部是目录，无实际影响）同样静默 EINVAL。
**菜单 1（施工单原菜单 1）**：bash 预处理 `>/dev/null` → 工作区 sink 文件。在菜单 0 可行后**不再必要**（多一层语义替换，复杂度高）。
**菜单 2（施工单原菜单 2）**：接受限制 + 文档化。在菜单 0 可行后不必要。

## 五、建议

批准"菜单 0"为独立施工单（sandbox crate，G0 范围，需顶层授权）：`add_landlock_rule` 增加文件级权限路径 + `/dev/null` 切换 + C 矩阵同款单测（或集成测试断言 `/dev/null` 规则 add 成功不再 warn）。修复后 TC-8 完整转绿（判定层已绿 + 执行层可写）。

## 附录 · 证据资产化

- C 矩阵测试源码：`docs/data/n1-landlock/ll_diag.c`（2026-08-30 自 .133:/tmp 归档——VM /tmp 重启即丢，复跑不重写）。
- 编译执行：`gcc -o /tmp/ll_diag /tmp/ll_diag.c && /tmp/ll_diag`（VM 需 landlock ABI available，kernel 7.0.0-30 实测通过）。
- 后续施工单：`docs/hearth-n1-sbx-landlock-file-rights-construction-order-v1.md`（设计稿，待顶层批准——含守门员 8 条补充约束，其中增 1 修正本报告：FS_RO 同病，READ_DIR 亦为目录专有位，FS_RO_FILE 需同修；增 2 修正：REFER 必须列入 DIR_ONLY 清单）。
