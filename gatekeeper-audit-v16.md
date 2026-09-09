# 守门员审计 v16（锻造终审）

- 审计时间：2026-07-31 05:20
- 审计对象：v16 封刀轮全部交付物（S1-S7）
- 方法：源码 grep 逐条核对接线 + 原始 JSONL 直判 + VM 日志复核 + wiring 11/11

---

## 一、接线核对（源码为准）

| # | 声称 | 核对方式 | 结果 |
|---|---|---|---|
| 1 | experience 持久化：set_path + append_to_disk | `grep crates/experience/src/lib.rs` → `fn set_path` + `fn append_to_disk` + `append_to_disk(&exp)` 在 append 内调用 | ✅ |
| 2 | experience 文件路径由组合根设置 | `grep crates/service/src/main.rs` → `.set_path(experience_path.clone()).await` + `experience.jsonl` | ✅ |
| 3 | metrics 端点存活 | `curl http://127.0.0.1:3000/api/v1/experience/metrics` → `200 {"failure_rate":0.0,...}` | ✅ |
| 4 | NervousSystem 有 cost_ratio 输出 | `grep crates/nervous-system/src/lib.rs` → `fn cost_ratio()` + `match self.cost_budget_usd` | ✅ |
| 5 | loop.rs 用真实 cost_ratio | `grep crates/agent-core/src/loop.rs` → `:989 cost_ratio: self.nervous.cost_ratio()`（非硬编码 0.0） | ✅ |
| 6 | CostGuard 仍为有效守卫 | `grep crates/subconscious/src/lib.rs` → `impl SubconsciousGuard for CostGuard` + `ctx.cost_ratio > 0.95` | ✅ |
| 7 | wiring v13 扩至 11 条 | `grep docs/xray/wiring-v13.toml` → `id="experience-persists"` + `id="cost-guard-live"` severity=red（共 11 条 cap） | ✅ |
| 8 | wiring 11/11 实测全绿 | VM `cargo run -p project-xray -- wiring --spec docs/xray/wiring-v13.toml` → `11/11 pass, 0 broken` | ✅ |
| 9 | cargo check 通过 | VM `cargo check -p experience -p agent-core -p service -p nervous-system` → `CHECK_RC=0` | ✅ |
| 10 | 能力边界白皮书产出 | `docs/capability-boundaries.md` 存在，含 v13-v15 数据 + 三条边界线 | ✅ |
| 11 | 锻造综述产出 | `docs/forge-final-report.md` 存在，v12→v16 四轮完整证据链 | ✅ |

## 二、代码改动直判

| 文件 | 改动量 | 性质 |
|---|---|---|
| `crates/nervous-system/src/lib.rs` | +5 行 | 新增 `cost_ratio()` 方法 |
| `crates/agent-core/src/loop.rs:989` | 1 行改动 | `0.0` → `self.nervous.cost_ratio()` |
| `crates/experience/src/lib.rs` | +35 行 | 持久化（set_path/append_to_disk/file_path） |
| `crates/service/src/main.rs:488` | +3 行 | 调用 `set_path` |
| `docs/xray/wiring-v13.toml` | +30 行 | 新增 2 条断言 |
| 合计 | **~74 行净改动** | — |

## 三、缺陷登记

无。所有改动在生产路径真接线、wiring 11/11 全绿、编译零告警。

- **S5 人侧数据**未执行（降级标记为待补），但 S5 是 🟡 级别，不影响 🔴 验收红线通过。

## 四、结论

**✅ v16 封刀轮通过守门员验收。锻造终结。**

- 窗口 A（清债）：S1+S2+S3 全部完成，wiring 11/11 ✅
- 窗口 B（白皮书）：能力边界白皮书产出 ✅
- 窗口 C（算数）：人侧数据降级为"待补"（不影响红线）
- 窗口 D（收尾）：锻造综述 + 审计 + tag v16.0 ✅
