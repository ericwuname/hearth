# CHANGELOG v19.0 — 深水轮

解剖 T13/T19 根因（planner 判定缺陷）+ 经验自适应开关 + deepseek 基线锚定新高。

## Added
- **S1/S2 解剖工具** `bench/anatomy_v19.py`：重跑单题 + 跑完立即 GET messages 录制完整事件流（关键：旧 session 历史已丢失，必须即时录制）。
- **T13/T19 解剖报告** `docs/t13-t19-anatomy-v19.md`：逐 tool_call 证据 + root cause。
- **S3 经验自适应开关**：`loop.rs` 经验搜索加 `consecutive_errors >= 3` 门控（弱模型降级通道，非强模型常驻噪音）+ wiring 断言 `experience-adaptive-switch`。
- **S5 基线锚定**：deepseek 固定序 20×2 = **37/40 = 92.5%**（历史新高）。

## Key Findings

### 🔴 T13/T19 root cause（六轮谜团解开）
```
agent 只 grep+read，从未 write/edit 就 self-report 完成（DONE ok=true）
根因：loop.rs:967 all_done 判定把 Read 节点的 Completed 视为任务完成
→ planner 缺陷，不是模型能力墙 → 换模型/加经验都无效的原因
```
- T13 解剖 PASS 对照 runner FAIL(TEST_FAIL)：代码根本没改
- T19 同模式（NO_GENERIC_FN：parse_positive 从未写入）

### 经验自适应开关
- v17（zhipu 弱模型）+20pt 有效 → v18（deepseek 强模型）-5pt 有害 → v19 门控：**失败≥3 才注入**
- wiring 12/12 全绿

### deepseek 基线
| 轮次 | 通过率 |
|---|---|
| v15 | 90.0% |
| v18 E0 随机 | 87.5% |
| **v19 S5 固定** | **92.5%** ✅ |

**季度体检基线正式发布：90% ± 5% 为正常波动。**

## Changed
- `loop.rs` 经验搜索路径加自适应门控（行为变更：首轮不再注入经验）
- `docs/xray/wiring-v13.toml` +1 断言（12 条）

## Known Debt（v20）
1. **planner all_done 判定修复**：task_graph 必须有 ≥1 个真实 Write 节点才允许 Done（T13/T19 根治）。
2. S4 zhipu 自适应子集验证（降级待补，deepseek 侧已无损失验证）。
3. T19 仍是唯一工具链盲区（v20 修复后重验）。
4. deepseek 余额 ~¥8（充足）。
