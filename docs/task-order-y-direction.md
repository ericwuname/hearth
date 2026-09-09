# Y 方向施工任务书 — P1 收口 + P2 CLI 六面 UX

> 基线：`docs/dev-scan-and-direction-y.md` 全部结论经源码核验
> 状态：扫描完成，两笔旧债已清。Y 方向 = 安全收口（P1）+ CLI 体验打磨（P2）
> 原则：不碰 G0/G1、零成本验证（replay/单测）、禁止假绿

---

## §0 审查补充（v1.0.3.1 定版，2026-08-01 18:40 执行窗口核验）

### P1 修正（执行顺序调整）
- Y1-1/Y1-3/Y1-4/Y1-5 全属实；Y1-5 实际 **25** 个未提交（非 23）
- **Y1-2 tag 移到 Y1-5 提交之后打**——tag 必须指向包含全部改动的 HEAD（含 key 修复+文档修正+UX），否则 tag 盖不住改动

### P2 重大修正（原"UX 0%"论断失真，已源码核验）
codex-cli 现有实现（`crates/codex-cli/src/`：main.rs 390 行 + client.rs 246 + repl.rs 224 + render.rs 69）：
- ✅ **指令面已实现**：15 子命令（chat/repl/sessions/history/approve/deny/cancel/status/replay/coverage/whoami/template/tools/civ/tasks）
- ✅ **呈现面已实现**：SSE 流渲染（Phase/Token/ToolCall/ToolResult/Done/Error 全彩色）
- ✅ **边界感面已实现**：`need_approval` 渲染 + approve/deny 命令 + REPL 内审批等待
- ❌ **上手面缺失**：无 `setup` 子命令（交互式配 key + 冒烟测试）
- ❌ **容错面缺失**：无 `resume <session_id>` 断点续传（有 history 但无续传）
- 🟡 **反馈面半成品**：Error 渲染 raw 消息，无结构化（发生了什么/原因/怎么修）
- 🔴 **codex-cli 零测试**：`tests/` 不存在、src 无 `#[test]`——所有改动无回归保护

**P2 修正为**：补 setup（上手）+ resume（容错）+ Error 结构化（反馈）+ 测试套件（replay/mock 零成本）——验证仍用 replay 三场景（正常/审批挂起/失败重试）。

### P3 决策（测试预算纪律）
- 真验需本机编译运行 service + 真 LLM 调用；用户「没钱测试」→ **P3 降级**：
  以 replay/mock 端到端为最终验收（零成本），智谱真验记为可选后续项

---

## P1：安全 + 收口（机械活，不触红线，~1h）

### Y1-1 🔴 硬编码 Agnes key 清零（最优先）

| 项 | 内容 |
|---|---|
| 位置 | `crates/service/src/main.rs:208` |
| 当前 | `"Agnes_API_KEY".unwrap_or("sk-8LBZ1Gtu...真实密钥")` |
| 目标 | 纯读 `AGNES_API_KEY` env，无 fallback 硬编码 |
| 验收 | `grep -r "sk-8LBZ1" crates/` 0 命中；service 编译通过；`AGNES_API_KEY` 未设时调用 agnes provider 返回 401（非 panic） |

### Y1-2 打 tag v22.0

| 项 | 内容 |
|---|---|
| 当前 | `git tag` 停在 v21.0 |
| 目标 | `git tag -a v22.0 -m "EPIC-B/C repaid; wiring 15/15; 212 tests; zero debt"` |
| 验收 | `git tag -l v22.0` 存在 |

### Y1-3 季度基线幽灵文档

| 项 | 内容 |
|---|---|
| 当前 | `quarterly-baseline*.md` 不存在，被 10+ 文档引用 |
| 选项 | A) 补真实文件 / B) 删引用 |
| 建议 | **B 删引用**——维护期不强制季度文档，季度体检产出的报告本身就够 |
| 验收 | `grep -r "quarterly-baseline" docs/` 0 命中（或保留 1 处指向体检报告） |

### Y1-4 Docker build 验证

```bash
cd ~/codex_work
docker build -t codex-rust:v22 .
docker run --rm -p 3000:3000 codex-rust:v22 &
sleep 10
curl http://localhost:3000/healthz
# 期望：返回 OK
```

| 验收 | /healthz 返回 200；容器内 sandbox 行为（landlock/seccomp）无 panic |

### Y1-5 提交 23 个未提交文件

| 项 | 内容 |
|---|---|
| 验收 | `git status --short` 为空或仅剩非项目文件 |

---

## P2：codex-rust CLI 六面 UX（Y 主线，~2h，零成本验证）

窗口群框架层 UX 六面已全闭。底层 codex-rust 的 `codex-cli` 子命令调用单 agent 的体验层仍是 0%。

六面映射：

| 面 | codex-cli 对应 | 改动 |
|---|---|---|
| **指令面** | CLI 子命令好记、help 清晰 | `codex-cli/src/main.rs` 子命令重命名（如有需要）+ `--help` 每命令有示例 |
| **呈现面** | 思考/工具调用/进度可见 | `send_message` 返回 SSE 流 → CLI 渲染彩色步骤（💭Plan / 🔍grep / ✏️write / 🔧bash / ✅DONE） |
| **反馈面** | 失败清晰报错 | CLI 输出结构化错误（发生了什么 / 原因 / 怎么修），非 raw JSON 或静默 |
| **上手面** | 首次引导 | `codex-cli setup` 交互式配 key + 冒烟测试 + 一句使用示例 |
| **边界感面** | 审批门可见 | SSE 流中 `need_approval` 事件 → CLI 渲染 ⏸️+ "原因：需审批 rm old.txt → codex-cli approve/reject" |
| **容错面** | 失败后重试 | `codex-cli resume <session_id>` 断点续传 |

**验收方式**：replay 模式跑 3 个场景（正常完成 / 审批挂起 / 编译错误重试），全部用本地 replay provider（零 token、零 LLM 调用）。

---

## 执行窗口排期

| 顺序 | 内容 | 耗时 |
|---|---|---|
| 1 | Y1-1 Agnes key 清零 | 5 分钟 |
| 2 | Y1-2 tag v22.0 | 1 分钟 |
| 3 | Y1-3 幽灵文档 | 10 分钟 |
| 4 | Y1-4 Docker build | 15 分钟 |
| 5 | Y1-5 提交文件 | 5 分钟 |
| 6 | Y2 CLI 六面 UX | ~2 小时 |
| 7 | Y3 智谱真验 | 30 分钟 |

**P1 五件事做完**——30 分钟内 Y 方向的安全和收口全清，codex-rust 项目进入"零债务、全提交、有 tag、Docker 可部署"的正式交付状态。**P2 CLI 六面 UX** 是 Y 的实质增量——补完即达成"单人好用"。
