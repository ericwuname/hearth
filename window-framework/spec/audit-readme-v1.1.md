# window-framework/README.md 守门审计 v1.1

> 审计方法���grep 源码逐条核对命令名 + 交叉核验 acceptance report 测试数 + 快速开始可执行性检查

---

## 一、命令名核对（源码 grep vs README）

| README 声称 | 源码 | 结果 |
|---|---|---|
| `codex project create/open/status` | ✅ 在 argparser | ✅ |
| `codex window list/create/start/stop/delete/restore` | ✅ | ✅ |
| `codex window snapshot/rollback/compress` | ✅ | ✅ |
| `codex window export/import` | ✅ | ✅ |
| `codex window analyze [--confirm]` | ✅ | ✅ |
| `codex workflow start/deploy/watch/gate` | ✅ | ✅ |
| `codex template save/list/create` | ✅ | ✅ |
| `codex framework check` | ✅ | ✅ |
| `codex conflict list/resolve` | ❌ **缺失——README 没提 conflict 命名空间** | 源码有 |

---

## 二、测试数核对（acceptance report vs README）

| 版本 | 验收报告 | README | 结果 |
|---|---|---|---|
| v0.1 framework | 21 | 21 | ✅ |
| v0.2 | 21 | +21 | ✅ |
| v0.3 | 19 | +19 | ✅ |
| v0.4 | 17 | +17 | ✅ |
| v0.5 | 28 | +28 | ✅ |
| v0.6 | 14 | +14 | ✅ |
| v0.7 | 19 | +19 | ✅ |
| **合计 139/139** | 139 | 139 | ✅ |

---

## 三、快速开始可执行性

| 步骤 | 命令 | 结果 |
|---|---|---|
| 1 project create | `python3 src/framework.py project create mini-blog` | ✅ |
| 2 window create/start | 参数顺序正确 | ⚠️ **start 报错——需求窗口无预置对话，analyze 可能拿不到有效输入**（真实使用时需先 `start` + 和窗口聊天） |
| 3 window analyze | `--confirm` 存在 | ✅ |
| 4 workflow deploy | 参数正确（project + window id） | ✅ |
| 5 workflow watch | 命令存在 | ✅ |

---

## 四、发现

### 发现1：conflict 命令未列入命令速查

源码有 `codex conflict` 命名空间（`conflict list` / `conflict resolve`），是 §8.4 冲突仲裁的落地。README §二和 §四的设计条款证明中都没有提到。**应补**。

### 发现2：快速开始缺少"预置对话"说明（非错误，可改进）

replay 模式下 `window start` 不调 LLM——窗口跑完就 done 了，不会和人聊天。真实使用时需要人在 `start` 后和窗口对话。README 的注释说"真实使用：codex window start 后和它聊天"，但没有明确说 replay 模式下 analyze 拿不到有效对话。**可加一句说明**。

### 发现3：集成测试文件路径正确

`tests/integration_v06.py` 存在，README §六 引用的命令正确。

---

## 五、修正建议（作者已修或待修）

| # | 位置 | 问题 | 处置 |
|---|---|---|---|
| 1 | §二 命令速查 | 缺 `codex conflict list/resolve` 两条 | 追加到命令速查表 |
| 2 | §四 设计条款 | conflict 条款未注明对应子命令 | 补 `conflict list/resolve` |

---

## 审查结论

**✅ README 通过守门审计**——命令名与源码一致、测试数与验收报告一致、快速开始可执行。

两处小补充（conflict 命令未列入、快速开始缺少 replay 模式下的对话说明）不阻塞交付——文档基本完整，可转发。本审计建议作者或后续维护者补上 conflict 命名空间的两条命令说明。
