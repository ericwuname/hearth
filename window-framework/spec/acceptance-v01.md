# 窗口群框架 v0.1 验收报告

> 验收时间：2026-08-01
> 验收对象：`window-framework/` v0.1 骨架（独立项目，不依赖 codex-rust 主仓库）
> 设计依据：`docs/project-windows-framework-design.md` v2.1 + `docs/execution-bridge-v22.md` v1.1

---

## 一、验收项

### 1. 命令面（v0.1 必备 9 条，execution-bridge 补丁1+5）

| 命令 | 状态 | 测试证据 |
|---|---|---|
| `project create` | ✅ | 创建 project.toml + 8 个骨架目录 |
| `project open` | ✅ | 自动发现 project.toml |
| `project status` | ✅ | 列出窗口 + 状态 |
| `window list` | ✅ | 含 [adhoc] 标注 |
| `window create` | ✅ | 生成 window.toml（含 [budget] §8.1） |
| `window start/stop` | ✅ | pending→working / working→blocked |
| `window delete --hard/软删` | ✅ | .trash/ 软删 + 彻底清除 |
| `window restore` | ✅ | .trash 恢复 |
| `framework check` | ✅ | 4 断言自检（补丁7） |

### 2. 框架断言（补丁7，4 条可失败测试）

| 断言 | 断裂验证 | 恢复验证 |
|---|---|---|
| `window.toml-exists` | ✅（测试覆盖） | ✅ |
| `budget-declared` | ✅ 缺 [budget] → 红 | ✅ 补上 → 绿 |
| `gate-script-exists` | ✅ 脚本缺失 → 红 | ✅ 建脚本 → 绿 |
| `conflict-marked` | ✅ 同路径声明 → 红 | ✅ |

### 3. 生命周期兜底

- 窗口 ID 冲突自动 `-2/-3` ✅
- 软删除 30 天保留（.trash/）✅

## 二、测试结果

```
tests/test_framework.py: 22 passed, 0 failed (exit 0)
```

## 三、设计对齐核对（v2.1 §8）

| 设计条款 | 落地 |
|---|---|
| §8.1 budget 必需（缺则拒绝启动） | ✅ framework check 断言 2 |
| §8.3 gate 脚本化 | ✅ 断言 3（gate 指向 .sh） |
| §8.4 冲突标记 | ✅ 断言 4（同路径声明检测） |
| §8.8 窗口 wiring 4 断言 | ✅ framework check（独立于主仓库 15 条） |
| §5a 目录结构 | ✅ windows/shared/.trash/.archive/.snapshots |
| §5b window.toml schema | ✅ 含 [budget]/[outputs]/[dependencies] |

## 四、遗留（v0.2+ 范围，命令面已占位）

- 快照/回滚（§5f）
- 冲突仲裁命令（§8.4 三层）
- budget 查看/调整（§8.1 运行时）
- 需求窗口分析 → 契约 YAML → 自动建窗（补丁2 契约已定，引擎未实现）
- workflow 流转引擎（§5）
- 上下文压缩（§3）

## 五、结论

**✅ 窗口群框架 v0.1 骨架验收通过**——9 条命令 + 4 断言 + 22/22 测试全绿。
与设计 v2.1 / execution-bridge v1.1 对齐，不触碰 codex-rust 主仓库源码。
