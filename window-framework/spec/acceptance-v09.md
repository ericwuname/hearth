# 窗口群框架 v0.9 验收报告（UX 收尾）

> 验收时间：2026-08-01
> 验收对象：`window-framework/` v0.9（呈现面 + 容错呈现 + polish）
> 计划依据：`window-framework/spec/plan-ux-v09.md`（v0.9.1 审查补充版）
> 执行方式：用户离线，全自主完成（审查→定版→执行→验收→报告）

---

## 一、验收红线核对

| 红线 | 判据 | 结果 |
|---|---|---|
| 🔴 | `window resume` 重跑已完成的步骤（非断点续传） | ✅ 从 last_turn+1 续编号，测试证实 max(new_turns) > last |
| 🔴 | `--verbose` 输出不含工具调用名称（仍黑箱） | ✅ 输出含 Plan/Read/Write/Bash/DONE + 工具摘要 |
| 🔴 | `codex status` 与 window.toml 不一致 | ✅ 测试证实窗口状态一致 |
| 🔴 | 框架 4 断言 + v0.1-v0.8 回归 | ✅ 全绿 139/139 |
| 🟡 | 非 tty `--verbose` 输出 raw traceback | ✅ 纯文本降级（无 ANSI/依赖） |

## 二、S1 呈现面：window run --verbose

- `Agent.run` 每轮产生 `StepLog{turn, phase, tool, summary, tokens, wall_ms}` ✅
- **持久化到 conversation.jsonl 的 meta.step_log**（补充1）——resume 依赖 ✅
- `window run --verbose` 渲染彩色步骤流（💭🔍✏️🔧❌✅）✅
- `window run` 不带 verbose = 等同 start（补充2，start/watch 保留兼容）✅
- replay 模式也写 step_log（demo 可渲染）✅

## 三、S2 容错呈现面

| 命令 | 功能 | 结果 |
|---|---|---|
| `window resume` | 断点续传（start_turn + 跳过空残留） | ✅ 测试证实不重跑 |
| `workflow deploy --last` | 用 last_analyze.json（不重新 analyze） | ✅ valid 建窗 / invalid 拒绝 |
| `workflow retry <stage>` | 只重置该 stage 窗口为 pending | ✅ 其余不动 |
| `last_analyze.json` | analyze 无论 confirm 与否都写快照（补充3） | ✅ |

## 四、S3 polish：codex status 全局聚合

- 一条命令聚合所有项目：窗口数/状态/轮数 ✅
- 与实际 window.toml 一致（测试验证）✅

## 五、v0.9.1 审查补充 5 项落地

| 补充 | 落地 |
|---|---|
| ① StepLog 持久化（meta.step_log） | ✅ |
| ② run/start/watch 兼容 | ✅ |
| ③ last_analyze 写入时机 | ✅（无论 confirm） |
| ④ resume 跳过不完整轮 + start_turn | ✅ |
| ⑤ 12 测试清单 | ✅ test_v09.py 21 项全覆盖 |

## 六、测试结果

| 套件 | 结果 |
|---|---|
| tests/test_v09.py（v0.9 新增） | ✅ **21 passed / 0 failed** |
| tests/test_v07.py（回归） | ✅ **19 passed / 0 failed** |
| tests/test_v06.py（回归） | ✅ **14 passed / 0 failed** |
| tests/test_v05.py（回归） | ✅ **28 passed / 0 failed** |
| tests/test_v04.py（回归） | ✅ **17 passed / 0 failed** |
| tests/test_v03.py（回归） | ✅ **19 passed / 0 failed** |
| tests/test_v02.py（回归） | ✅ **21 passed / 0 failed** |
| tests/test_framework.py（回归） | ✅ **21 passed / 0 failed** |
| **合计** | ✅ **160/160** |

## 七、UX 六面总结

```
上手面     ████████████  （project create + window + analyze + deploy 四步）
反馈面     ████████████  （每命令有输出，失败有修复建议）
边界感     ████████████  （沙箱 + gate + budget 上限）
呈现面     ████████████  （window run --verbose 步骤流）
容错面     ████████████  （resume / deploy --last / workflow retry）
polish     ████████████  （codex status 全局聚合）
                          六面全闭，UX 进入维护期
```

## 结论

**✅ 窗口群框架 v0.9 验收通过——UX 六面全闭。**
160/160 测试全绿（新增 21 + 回归 139），红线 4+1 全过。
窗口从"黑箱"到"agent 思考可见 + 断点续传 + 一键看全局"。
框架 v1.0（功能）+ UX v0.9（体验）双定版，进入维护期。
