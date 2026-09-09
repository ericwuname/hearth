# Y 方向验收报告（P1 收口 + P2 CLI 六面 UX）

> 日期：2026-08-01 | 基线：HEAD `06795df` → 验收后 `0153a26` + tag `v22.0`（`c0f0f52`）
> 任务书：`docs/task-order-y-direction.md`（§0 审查补充后定版）
> 原则：不碰 G0/G1、零成本验证（replay/mock/单测）、禁止假绿

---

## 一、P1 安全 + 收口（5 项）

| 项 | 状态 | 证据 |
|---|---|---|
| **Y1-1 🔴 Agnes key 清零** | ✅ | `main.rs:207-208` 硬编码真实 key → 纯读 `AGNES_API_KEY` env（无 fallback）；`grep -r "sk-8LBZ1" crates/` **0 命中** |
| **Y1-2 tag v22.0** | ✅ | `git tag -l v22.0` 存在（指向 `c0f0f52`，含全部 Y 改动） |
| **Y1-3 季度基线幽灵文档** | ✅ | 选 B 删引用：规程 §7 挂空引用 → 指向真实体检报告 `acceptance-final-epic-bc.md`（Q3 基线值 wiring 15/15 + 214 passed）；roadmap N1 关闭；agenda #16 ✅ |
| **Y1-4 Docker** | 🟡 部分 | ✅ 补 `.dockerignore`（原 `COPY . .` 带 24M .git 进上下文）；❌ **build 未验证**——本地 Docker Desktop daemon 无法启动（GUI 交互）+ VM docker hub 不可达（registry-2.docker.io 连接失败，与 MEMORY 一致）。按禁止假绿纪律如实记录，转用户环境待办 |
| **Y1-5 提交全部文件** | ✅ | `git status --short` = **0**（30 项全提交，2 commits） |

## 二、P2 codex-cli 六面 UX（审查修正后）

**审查发现**：任务书原"UX 0%"失真——codex-cli 已有指令面（15 子命令）/呈现面（彩色 SSE）/边界感面（approve+deny）。**真实缺口 = 上手面 + 容错面 + 反馈面半成品 + 零测试**。定版修正后施工：

| 面 | 改动 | 实测 |
|---|---|---|
| **上手面** | `codex setup`：交互配 URL/key + 写 `.env` + `/healthz` 冒烟 + 使用示例 | ✅ 交互→冒烟 200（连 VM 真 service）→示例输出 |
| **容错面** | `codex resume <id> [goal]`：状态检查（done/cancelled 拒绝续传）+ 复用 sid 续跑 | ✅ `resume session (status=created)` → 续跑 SSE → Done |
| **反馈面** | `classify_error` 5 类归因（连不上/401/404/超时/其他）→ 三行结构化（发生了什么/原因/怎么修） | ✅ 实测"连不上→原因→怎么修"三行 |
| **边界感面补强** | `need_approval` 事件在 Chat/Resume 也渲染（原只 REPL） | ✅ `⛔ rm old.txt — 用 approve <sid> <aid> 审批…` |
| **呈现面/指令面** | 已有，未破坏（回归 214 全绿） | ✅ |
| **测试保护** | SSE 解析重构 `parse_sse_lines` 纯函数（跨 chunk 续接）+ **+8 单测**（解析 4 + 归因 4） | ✅ 8/8 过 |

## 三、验收证据

### VM 门禁（全量）
```
FMT_RC=0  CLIPPY_RC=0（workspace 全量 -D warnings）  TEST_RC=0  BUILD_RC=0
test result: 214 passed（51 个 suite——比上轮 206 +8，恰为 codex-cli 新测试）
```

### mock 三场景（零 token，`window-framework/tests/_cli_mock.py`）
```
场景1 正常完成:  session mock-sess-1 → ◆plan → ⚙bash → ✓结果 → ✓Done (3 steps)
场景2 审批挂起:  ⚙bash rm old.txt → ⛔ 用 approve/deny 审批提示
场景3 编译错误:  ⚙bash cargo build → ✗ compile error: unresolved import
场景4 断点续传:  resume session (status=created) → 续跑 → Done
场景5 上手引导:  交互配 URL/key → 已写入 .env → ✓ 冒烟测试通过（连真 service 200）
场景6 错误结构化: ✗ 连不上 codex service / 原因：connection refused / 怎么修：先启动 service…
```

### wiring 静态核验（改动后未失）
15 capability / 24 chains / 0 缺失（EPIC-B/C 后基线保持）

## 四、P3 真验决策（测试预算纪律）

任务书 §0 定版：P3 智谱真验**降级**——以 replay/mock 端到端为最终验收（零成本，已完成）；真 LLM 端到端记为后续可选（用户「没钱测试」纪律）。

## 五、诚实披露

- **Y1-4 Docker build 未验证**：环境限制（本地 daemon 不可用 + VM hub 不可达），Dockerfile+.dockerignore 已交付，build 转待办——**不假绿**
- 施工中发现并修复 1 个既有 bug：SSE 解析重构时暴露跨 chunk 丢 event_type 状态（重构引入，测试捕获并修复）
- P2 修正了任务书"UX 0%"的错误论断（原实现已覆盖三面）

## 六、交付

- commit `c0f0f52`（P1+P2）+ `0153a26`（其余文件存档）
- tag `v22.0`——codex-rust 首个"零债务、全提交、有 tag、key 干净"正式交付状态
- `window-framework/tests/_cli_mock.py`——CLI UX 验收可复现 mock
