# v23 阶段一验收报告：WP-0a + 红队 P0 + 文档修正

> 日期：2026-08-02 | 基线：HEAD `1255498` → 验收后 `df41bc7`
> 依据：`top-level-plan-v23.md` + `-supplement.md` + `audit-findings-v22.md` + `v23-execution-order.md`
> 性质：执行窗口一轮自主施工——**WP-0a 全完成 / WP-0 未开工（标记下批）/ 红队 P0 快修 / 文档修正**

---

## 一、WP-0a：审批原语修复（RT4 三缺陷，全完成 ✅）

| 修 | 内容 | 证据 |
|---|---|---|
| **R1 id 强校验** | `dispatcher.rs` `resolve_approval` 加 `approval_id` 参数；Pending 项的 `approval_id` 不匹配必拒（修 RT4-③ 任意响应顶替任意审批） | 新测试 `test_wp0a_r1_wrong_approval_id_rejected`：错 id → mismatch 错误 + 状态保持 Pending ✅ |
| **R2 一次性消费** | 已消费的 pending 重复提交必拒（防重放） | 新测试 `test_wp0a_r2_double_submit_rejected`：二次提交 → no-pending 错误 ✅ |
| **R3 契约统一** | `codex-cli` 发 `{"approval_id", "decision": "approve"/"deny"}`（修 RT4-② 旧 `approved:bool` 导致 422，approve/deny 子命令完全不可用） | codex-cli 8/8 测试过 + 门禁编译过 ✅ |

**门禁**：现有审批测试**零改动**仍全过（test_approval_flow/deny/isolated）；tool-runtime **11/11**（原 9 + 新 2）。

## 二、红队 P0 快修（window-framework，全完成 ✅）

| 项 | 修复 | 证据 |
|---|---|---|
| **WT6** stale working 假完成 | working 重置 pending 后置 `all_done=False`（修单窗口 stage 被自动 gate 判 done 而窗口从未运行） | 回归测试：单窗口 stage + working → 引擎真实运行窗口 → state=done + rc=0 ✅ |
| **WT17** prompt 注入破坏 TOML | prompt 写入改 `json.dumps(..., ensure_ascii=False)`（完整转义换行/引号/反斜杠） | 回归测试：含 `\n`/`"`/`\` 的 prompt 写入后可被 tomllib 读回且原文一致 ✅ |

**回归**：window-framework **180/180**（v10 17→20，+3 WT6/WT17）。

## 三、文档失真修正（CT3/CT6/CT8，全完成 ✅）

- **CT3**：version-iteration-manual L233 时点快照标注（205/14 是 v22 验证期数据，当前 214/15/15）
- **CT6**：acceptance-final-epic-bc.md 追加"红队复核"段——如实标注 /readyz 半修（无 store 永远 200）、civ 落盘半修（writer None 静默丢弃）、quarterly-baseline 决断有效
- **CT8**：README 全量回归命令补 `v09 v10`（177→180 可复现）

## 四、门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **214 tests 零回归**（+ tool-runtime 11 + codex-cli 8） |
| window-framework | ✅ **180/180** |

## 五、⚠️ WP-0 未开工（诚实披露，非假绿）

**WP-0（通用交互原语）本轮未施工**——原因：它是 4h 连续工程（agent-types 新类型 + agent-core 通用暂停管道 + service 通用路由 + 审批迁移为 kind="approval" + dummy kind 0 行验证），**半途而废比不做更糟**：审批管道若处于迁移中间态，现有审批会失效（违背"现有 214 tests 不得回归"硬红线）。

**下一批第一项 = WP-0**（WP-0a 前置已完成，可直接开工）。v23 其余 WP-1~WP-10 与红队剩余项（RT2 civ 缺失路径 / RT3 seccomp / RT6 限流 / RT9 会话持久化 / WT1 bash 沙箱 / N7 密钥串用复核 / CT1 wiring AST 强化）一并挂账。

## 六、红队对应项闭环状态

| 红队项 | 状态 |
|---|---|
| RT4 审批门绕过（①②③） | ✅ 全闭环（WP-0a R1/R2/R3） |
| WT6 stale-working 假完成 | ✅ 已修 + 回归 |
| WT17 prompt 注入 | ✅ 已修 + 回归 |
| CT3/CT6/CT8 文档失真 | ✅ 已修 |
| RT1/RT2/RT3/RT6/RT9/WT1/N7/CT1 | 🟡 挂账下批（v23-execution-order §三） |

## 七、交付

- commit `df41bc7`（WP-0a + 红队 P0 + 文档 + 定版）
- `docs/v23-execution-order.md`（执行定版 + 挂账清单）
- 下一批：WP-0 全量 → WP-1 → WP-2 → ...
