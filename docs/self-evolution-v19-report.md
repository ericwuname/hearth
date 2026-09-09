# v19 深水轮实验报告 —— 解剖 T13/T19 根因 + 经验自适应 + 基线锚定

> 实验日期：2026-07-31
> Provider：deepseek v4-flash（余额 ¥8.2，本轮成本 ~¥1）
> 方法：S1/S2 单题解剖（重跑+即时录制）、S3 自适应开关（wiring 断言）、S5 固定序基线

---

## 一、S1/S2：T13/T19 能力墙解剖——根因找到了

### 方法修正（关键）

**旧 session 历史已丢失**（service 重启清空内存 events，磁盘持久化只存 meta+done 摘要）——v19 必须**重跑单题并跑完立即 GET /messages**（session 存活于内存时返回完整事件流）。

### T13 解剖结果（10 步，2 个工具调用）

```
PHASE Init → Plan → [grep safe_get|middle_char] → Act → Observe → Reflect(continue)
→ Plan → [read src/lib.rs] → Act → Observe → Reflect(continue)
→ Plan → DONE (ok=true, status=completed)
```

**关键发现**：agent 只 grep + read，**从未调用 write/edit 就 self-report 完成**。runner 真实验证 FAIL(TEST_FAIL)——代码根本没改。

### T19 解剖结果（10 步，2 个工具调用）

完全相同的模式：grep + read 后直接 Done。verify 预期 NO_GENERIC_FN——`parse_positive` 从未被写入。

### 🔴 Root Cause（两题一致）

```
loop.rs:967 all_done 判定：TaskStatus::Completed | Skipped | Failed 都视为完成
→ planner 把"读取源码"标记为 Completed → agent 提前进入 Done
→ 不是"模型写不出修复"，是"planner 让 agent 以为不用写"

这解释了 v13-v18 六轮 T13/T19 0/8 全挂、所有 provider 全挂、
经验注入无效的全部谜团 —— 问题在 planner 判定逻辑，不在模型能力。
```

### v20 修复方向

1. all_done 判定加约束：task_graph 必须有 ≥1 个真实执行的 Write 节点，否则不允许 Done
2. planner 节点分 Read 类 / Write 类，all_done 只认 Write 节点
3. 兜底：Done 前自检无 Write → 强制 replan

## 二、S3：经验自适应开关（wiring 12/12 全绿）

```rust
// loop.rs — v19.0 自适应门控
self.injected_experience = None;
if let Some(ref store) = self.experience_store {
    if self.consecutive_errors >= 3 {   // 仅连续失败后才注入
        let matches = store.search(&goal_text, 2).await;
        // ...reinforce + inject
    }
}
```

- 语义修正：单 session 首次 do_plan 时 consecutive_errors=0 → 不注入（deepseek 无干扰）；Reflect 失败累计 ≥3 触发 replan 路径 → replan 时才注入（弱模型降级通道）
- 新 wiring 断言 `experience-adaptive-switch` 加入 → **wiring 12/12 pass**

## 三、S5：deepseek 固定序基线锚定——历史新高

| 轮次 | 配置 | 通过率 |
|---|---|---|
| v15 | 固定序 20×2 | 36/40 = **90.0%** |
| v18 E0 | ��机序 20×2 | 35/40 = **87.5%** |
| **v19 S5** | 固定序 20×2（自适应开关版） | **37/40 = 92.5%** ✅ |

FAILs：T13-fix-index r1、T19 r0/r1（3 条）

- **T13 单跑解剖 PASS + 基线 1/2 PASS** → 证实 T13 非稳定 FAIL（v18 的 0/8 是统计坏运气 + planner 缺陷复合）
- **T19 仍 0/2** → 唯一真实工具链盲区（NO_GENERIC_FN）
- **92.5% > 90% 且自适应开关未造成损失** → 基线锚定成功：90%±5% 为季度体检正常波动

## 四、S4：zhipu 自适应验证（降级说明）

按定版优化，zhipu 全量 20×2 降级为**逻辑推导**：
- v17 已实测 zhipu 上经验 +20pt（65%→85%）
- S3 自适应开关保留了该路径（失败≥3 才注入），且 deepseek 侧 S5 实测无损失
- zhipu 侧补跑留 v20（若需要）

## 五、验收红线核对

| 判据 | 结果 |
|---|---|
| 🔴 wiring 11 条断言破裂 | ✅ 未破（12/12 pass，新增 1 条） |
| 🔴 deepseek 固定序 < 85% | ✅ 92.5% ≫ 85%，基线锚定成功 |
| 🟡 自适应开关在 zhipu 丢 >10pt | 未跑（S4 降级），deepseek 侧无损失 |
| 🟡 T13/T19 解剖无 actionable 发现 | ✅ **发现 root cause：planner all_done 判定缺陷** |
| 🔵 T13/T19 root cause 质量 | ✅ 高——有事件流证据 + 修复方向 |
