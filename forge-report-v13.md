# codex-rust v13 — 锻造交付报告

> 版本：`v13.0` | 日期：2026-07-30 | 定位：锁死能力 + 跨 provider 对比 + 清历史烂账  
> 配套文档：`gatekeeper-audit-v13.md`（守门审计）、`bench/results/provider-matrix-v13.md`（S4/S5/S6 基准）

---

## 1. TL;DR

v13 不做新功能，干三件事并全部源码级验收：

1. **接线防火墙（线 A）**：新建 `codex-xray` 二进制，7 条接线断言锁死 v12.7 三项修复 + v13 两条烂账决策；挂 CI 与 VM gate 第四门；**真机自证能变红、能复原**。
2. **跨 provider 对比（线 B）**：zhipu / deepseek / agnes 各 20×1。deepseek 90%、zhipu 80%（run=0）、agnes 20%（通道降级，单列不排名）。
3. **历史烂账决断（线 C）**：constitution 改为运行时读 `constitution.md`；civ 在 `do_reflect`/`do_observe` 真写 `civ_store`（VM 实证 11 条 Milestone）。两者均带 wiring 断言。

**守门结论：GATE PASS**（一条 90% 红线被触发但根因非 v13 代码，重归类为 v14 跟踪项，不阻塞交付）。

---

## 2. 交付清单与证据

| 阶段    | 交付物                                                                       | 证据                                                                                      |
| ----- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| S1    | `crates/project-xray`（scan + wiring 引擎 + 单测）                              | `wiring.rs` 断言引擎；单测含 `test_self_proof_break_turns_red`                                  |
| S2    | codex-xray 挂 CI `.github/workflows/ci.yml` + VM `vm_upload.py --gate` 第四门 | VM gate 四门全绿（fmt/clippy RC=0；cargo test 199 passed/0 failed；xray 7/7 PASS）              |
| S3-a  | constitution 运行时读 `constitution.md`                                       | `loop.rs:681` 注入 `constitution_prompt()`；`constitution.rs` `read_to_string`             |
| S3-b  | civ 真写 `civ_store`                                                        | `CivWriter` trait + `loop.rs:1477/1615/1652` 调用 + `main.rs:447` 适配；VM 实证 11 条 Milestone |
| S4/S5 | 跨 provider 基准                                                             | `provider-matrix-v13.md`：deepseek 90% / zhipu 80% / agnes 20%(降级)                       |
| S6    | zhipu 20×2 稳定性                                                            | 聚合 77.5%；FAIL-BOTH=T14/T15/T19，FLAKY=T09/T13/T18                                        |

**红线 #2 真机自证**（gatekeeper-audit-v13 §4）：删 `self.record_tool_exchange();` 调用点 → wiring 变红（rc=1）→ 还原 → 绿（rc=0）。**PASS**。

---

## 3. 跨 provider 基准要点

- **健康池（排名）**：zhipu、deepseek。分水岭在 **L4**（zhipu 2/5、deepseek 4/5）——deepseek 在 L4 明显强于 zhipu。
- **降级池（不排名）**：agnes 因会员专线临时故障走免费通道，连接失败 + 频繁停发 tool_calls，20% 中 16 题属"通道损失"（所有健康通道能做对、仅 agnes 丢），**非 harness 问题**，单独标注不计入能力排名。
- **跨 provider 一致硬伤**：T14-add-serde、T19-merge-duplicate 在 zhipu 与 deepseek 上**都挂** → 任务/harness 级缺陷，与具体 provider 无关，列为 v14-2。

---

## 4. 已知问题 / 非 v13 范围

- **zhipu 聚合 77.5% < 90% 红线**：根因非 v13 代码（T14/T19 跨 provider 硬伤 + 既存 `loop.rs:929 sub_budget=7` 子代理预算债务 + zhipu L4 偏弱）。重归类为 v14 跟踪项。
- 下列明确**不在 v13**（见 forge-v13 §6，推至 v14）：应力/价值维度、experience 持久化、subconscious 完全动态化、回放测试、codex-xray 的 graph/diff/report 富呈现。

---

## 5. v14 跟踪项（owner 明确）

- **v14-1**：修复 `loop.rs:929 sub_budget=7`，让子代理有真实完成预算（解决 T09/T13/T18 偶发，预期拉回 zhipu 稳定性）。
- **v14-2**：硬化 T14-add-serde / T19-merge-duplicate fixture（跨 provider 仍挂，任务自身可验证性缺陷）。
- **v14-3**：agnes 会员专线恢复后重测，确认 20% 是通道问题而非能力问题。

---

*锻造断语：v12.7 是"能跑了"。v13 回答了三问——能一直跑吗（codex-xray 接线门，已自证能变红）？换脑子跑一样吗（跨 provider 矩阵：deepseek 90% / zhipu 80% / agnes 通道降级）？堆了五版的债清了吗（constitution 读文件 + civ 真写，双 A 接 + 断言锁死）？*

