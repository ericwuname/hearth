# v24-post 全面测试规划 — 零预算版（免费提供者 + 全量覆盖）

> 基线：v23.0 封版（240 tests）+ P0/P1 全闭环（12/12）+ P2 剩余 2 个开放项
> 约束：不花钱——付费 LLM 全用智谱（免费 token）或 agnes（免费通道）替代 deepseek
> 策略：先修后测——两个 P2 开放项修完再跑全量回归

---

## 〇、前置修复（P2 未闭环项，阻塞全量测试）

| 项 | 问题 | 修复 | 估时 |
|---|---|---|---|
| **P2-issue-1 🔴** | xray 哈希锁用 `DefaultHasher`（rustc 版本相关），VM/Docker 必红 | 改用 `sha2::Sha256` 或自写 FNV-1a 稳定哈希 | 30min |
| **P2-issue-2 🟡** | `cmd_window_snapshot` 漏拷 `outputs/` 目录 | 补 `shutil.copytree` 与 `_auto_snapshot` 一致 | 15min |

**验收**：VM 上 `cargo test --workspace` 全绿 + window-framework 全量回归全绿。

---

## 一、测试矩阵总览（6 层，按成本从零到低排列）

```
第1层  静态零成本（本地秒级）      ~0s, ¥0
第2层  单元零成本（本地分钟级）    ~2min, ¥0
第3层  VM 门禁（VM 分钟级）       ~5min, ¥0
第4层  免费提供者轻量（zhipu）    ~20min, ¥0（免费 token）
第5层  免费提供者全量（zhipu）    ~2h, ¥0（免费 token）
第6层  免费提供者马拉松（agnes）  ~4h, ¥0（免费通道慢但免费）
```

---

## 二、第1层：静态零成本（本地，~0s）

| # | 测试 | 命令 | 判据 |
|---|---|---|---|
| S01 | wiring 完整度 | `grep -c '\[\[capability\]\]' docs/xray/wiring-v13.toml` | 15 |
| S02 | Agnes key 零攻击面 | `grep -r 'sk-8LBZ1' crates/` | 0 hits |
| S03 | 任何硬编码 key | `grep -r 'sk-\(28d737\|aedbf1\|proj-bPk\|ark-9510\|AQ\.Ab8RN\)' crates/` | 0 hits |
| S04 | agent-core 不认 kind | `grep -rn '"approval"\|"clarification"' crates/agent-core/src/` | 0 分支 |
| S05 | Observer ��立 crate | `ls crates/observer/Cargo.toml` | 存在 |
| S06 | agent-core 不 import observer | `grep -r 'observer' crates/agent-core/Cargo.toml crates/agent-core/src/` | 0 hits |
| S07 | Observer 禁 LLM | `grep -r 'llm\|chat\|generate' crates/observer/src/` | 0 hits（排除注释） |
| S08 | version manual 已补 v23 | `grep 'v23' docs/version-iteration-manual.md \| head -3` | 含 v23 |
| S09 | git 干净 | `git status --short \| wc -l` | 0 |
| S10 | window-framework provider 路由 | `grep 'PROVIDERS' window-framework/src/framework.py \| head -1` | 存在 |

---

## 三、第2层：单元零成本（本地，~2min）

| # | 测试 | 命令 | 判据 |
|---|---|---|---|
| U01 | codex-rust 全量单测 | `cargo test --workspace` | 240+ passed / 0 failed |
| U02 | codex-rust fmt | `cargo fmt --all -- --check` | 0 |
| U03 | codex-rust clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| U04 | codex-rust release build | `cargo build --release` | 成功 |
| U05 | window-framework 全量 | `cd window-framework && AGENT_MODE=replay python3 tests/run_all.sh` | 208+ passed |
| U06 | window-framework replay 全链路 | 第5层做 | — |

---

## 四、第3层：VM 门禁（VM，~5min，零 token）

| # | 测试 | 命令 | 判据 |
|---|---|---|---|
| V01 | VM 全量单测 | `ssh wutao@192.168.220.131 'cd ~/codex_dev && source ~/.cargo/env && cargo test --workspace'` | 全绿（特别关注 xray 哈希锁修复后） |
| V02 | VM fmt | `cargo fmt --all -- --check` | 0 |
| V03 | VM clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| V04 | VM wiring | `cargo run -p project-xray -- wiring` | 15/15 |
| V05 | VM build release | `cargo build --release` | 成功 |
| V06 | P0 回归门禁 | `CODEX_VM_PW=123456 python .workbuddy/v22_regression_gate.py P0-1 P0-2 P0-3 P0-4` | 全 PASS |
| V07 | P1 回归门禁 | `CODEX_VM_PW=123456 python .workbuddy/v22_regression_gate.py P1-1 P1-2 P1-3` | 全 PASS |
| V08 | P2 门禁（限流/gate/路由） | `CODEX_VM_PW=123456 python .workbuddy/v22_regression_gate.py P2-1 P2-3` | 全 PASS |

---

## 五、第4层：免费提供者轻量（zhipu，~20min，免费 token）

智谱有大量免费 token。虽然比 deepseek 慢，但不花钱。

| # | 测试 | 命令 | 判据 |
|---|---|---|---|
| Z01 | zhipu 冒烟 T00 | `cd bench && BENCH_PROVIDER=zhipu python3 runner.py run T00-smoke --run 1` | VERIFY_PASS |
| Z02 | zhipu L1-L3 轻量 | `BENCH_PROVIDER=zhipu BENCH_BUDGET=20 python3 runner.py batch --runs 1`（限制只跑 L1-L3 题） | ≥ 15/20 |
| Z03 | zhipu 应力场 24 次 | `STRESS_PROVIDER=zhipu STRESS_RUNS=3 python3 bench/stress_v15.py` | 0 panic |
| Z04 | zhipu 回放验证 | 用 zhipu 做 provider 跑 replay fixtures（如果 replay provider 不支持 zhipu，跳过） | 不影响——replay 不调 LLM |
| Z05 | zhipu analyze 首胜率 | `cd window-framework && AGENT_MODE=real FW_PROVIDER=zhipu python3 src/framework.py window analyze test-proj win-req` | 一次成功 |

---

## 六、第5层：免费提供者全量（zhipu，~2h，免费 token）

| # | 测试 | 命令 | 判据 |
|---|---|---|---|
| Z10 | zhipu 20×2 全量基准 | `BENCH_PROVIDER=zhipu BENCH_BUDGET=30 python3 bench/runner.py batch --runs 2` | ≥ 70%（zhipu 历史基线 65-85%，不强求 90%） |
| Z11 | zhipu 单题重跑（失败题） | 取 Z10 失败题 × 3 遍 | 确认稳定性 |
| Z12 | zhipu codex-cli 端到端 | `codex-cli session create` → `send "fix T02"` → `get status` | 全链路通 |

---

## 七、第6层：免费提供者马拉松（agnes，~4h，免费通道慢）

agnes 专线不可用走免费通道，单次调用 ~6-8s，慢但免费。**只在 zhipu 全部跑完后考虑，不优先。**

| # | 测试 | 命令 | 判据 |
|---|---|---|---|
| A01 | agnes 冒烟 T00 | `BENCH_PROVIDER=agnes python3 runner.py run T00-smoke --run 1` | 不 panic/超时 |
| A02 | agnes L1-L2 轻量（~1h） | `BENCH_PROVIDER=agnes BENCH_BUDGET=15 python3 runner.py batch --tasks T00,T01,T02,T03,T04,T05,T06,T07,T08 --runs 1` | 不崩即可，不强求通过率 |
| A03 | agnes 应力场（~2h） | `STRESS_PROVIDER=agnes STRESS_RUNS=1 python3 bench/stress_v15.py` | 0 panic |

---

## 八、Observer 季度体检（零 token，VM 本地）

| # | 测试 | 判据 |
|---|---|---|
| O01 | G3 确定性指标 | `cargo test -p observer test_deterministic_metrics_same_twice` | 通过 |
| O02 | 5 条规则全触发 | `cargo test -p observer test_v24_quarterly_all_rules_trigger` | 通过 |
| O03 | 熔断触发 | `cargo test -p observer test_circuit_sandbox_violation` | 通过 |
| O04 | L2 fail-closed | `cargo test -p observer test_l2_fail_closed_on_seq_gap` | 通过 |
| O05 | L3 evidence 强制 | `cargo test -p observer test_l3_finding_evidence_required` | 通过 |

---

## 九、window-framework 全量（零 token，本地 + VM）

| # | 测试 | 命令 | 判据 |
|---|---|---|---|
| W01 | 全量回归 | `cd window-framework && AGENT_MODE=replay python3 -m pytest tests/ -v` | 208+ passed |
| W02 | replay 全链路 | analyze→deploy→workflow→check 连跑 | 各环节不崩 |
| W03 | framework check | `python3 src/framework.py framework check test-proj` | 4/4 PASS |
| W04 | 快照/回滚验证（P2-6 修复后） | 手动 `window snapshot` → 删 outputs → `window rollback` → outputs 恢复 | outputs 存在 + 内容一致 |

---

## 十、执行顺序

```
第0步（修复，45min）：
  P2-issue-1 xray 哈希 → P2-issue-2 snapshot outputs

第1轮（本地，~5min，全部零成本）：
  S01-S10 静态检查 → U01-U06 单元门禁

第2轮（VM，~10min，零 token）：
  V01-V08 VM 门禁 → O01-O05 Observer 体检 → W01-W04 window-framework

第3轮（zhipu 免费，~2.5h 机器时间）：
  Z01-Z05 轻量（20min）→ Z10-Z12 全量（2h）

第4轮（agnes 免费，~4h，可选）：
  A01-A03（如果第3轮全部通过且还有时间/意愿）
```

---

## 十一、目标通过标准

| 层级 | 标准 |
|---|---|
| 第1-2层（静态+单元） | **100%**——必须全绿，任何 red = 阻塞 |
| 第3层（VM 门禁） | **100%**——必须全绿 |
| 第4层（zhipu 轻量） | **冒烟必须过**，基准不强求（zhipu 历史 65-85%，不设及格线） |
| 第5层（zhipu 全量） | ≥ 70% 即可，低于则记录弱点 |
| 第6层（agnes） | **不崩就行**——通过率不定 |
| Observer | 5/5 全过 |

---

## 十二、报告产出

跑完后输出 `docs/audit-full-v24-post.md`：
- 逐层通过率（第1层 X/10, 第2层 X/6, ...）
- 免费提供者对比表（zhipu vs agnes 通过率/成本/时长）
- 新发现的问题（如有）
- 与 v22 红队发现的对照（P0/P1 无回归？P2 剩余项？）