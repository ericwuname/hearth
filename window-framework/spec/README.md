# 窗口群框架 v0.1（骨架）

> 独立项目——不依赖 codex-rust 主仓库源码。设计依据：
> `../docs/project-windows-framework-design.md` v2.1 + `../docs/execution-bridge-v22.md` v1.1。

## 结构

```
window-framework/
├── src/framework.py          # CLI 实现（9 条 v0.1 命令）
├── tests/test_framework.py   # 集成测试（命令 + 4 断言）
└── spec/                     # schema 文档（占位）
```

## 命令面（v0.1 必备 9 条）

```bash
export CODEX_PROJECTS_ROOT=~/.codex-projects
python3 src/framework.py project create <name> [--type software|writing|data|research|ops]
python3 src/framework.py project open [name]
python3 src/framework.py project status [name]
python3 src/framework.py window list <project>
python3 src/framework.py window create <project> --name <id> [--role R] [--max-steps N] [--max-cost C] [--provider P]
python3 src/framework.py window start <project> <id>
python3 src/framework.py window stop <project> <id>
python3 src/framework.py window delete <project> <id> [--hard]
python3 src/framework.py window restore <project> <id>
python3 src/framework.py framework check <project>   # 4 断言自检
```

未实现（v0.2+，命令面占位）：`window snapshot/rollback`、`conflict list/resolve`、
`window budget`、`window export/import`、`template`、`workflow`。

## 框架断言（补丁7，`framework check`）

| 断言 | 断裂条件 |
|---|---|
| `window.toml-exists` | windows/*/ 缺 window.toml |
| `budget-declared` | window.toml 缺 [budget] 段（§8.1） |
| `gate-script-exists` | outputs.gate 指向的脚本不存在（§8.3） |
| `conflict-marked` | 两窗口声明同一 outputs 路径（§8.4） |

与 codex-rust 主仓库 `docs/xray/wiring-v13.toml`（15 条）**平行管理**，不混入。

## 验收

```bash
cd window-framework && python3 tests/test_framework.py
# 期望：全部 PASS，exit 0
```
