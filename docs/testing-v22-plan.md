# v22 深测计划 — 基准 + 应力 + 新 provider + 效率诊断

> 定位：v21 测试跑通了编译门/服务/接线/持久化/provider 五道关口，
> 发现 3 个值得深挖的点。v22 补跑 T6 基准 + T7 应力，然后针对新数据加一道诊断。

---

## §1 v21 测试结果摘要

| 项目 | 结果 | 评价 |
|---|---|---|
| fmt | ❌ RC=1（8 diff） | `cargo fmt --all` 一条命令的事 |
| clippy | ✅ 0 warning | — |
| test | ✅ **205/0** | +25 vs v20 的 180 |
| wiring | ✅ 14/14 | 含防火墙自证（故意断线→红→绿） |
| 服务冒烟 | ✅ 启动+T00 PASS | — |
| 经验持久化 | ✅ 88→88 不丢 | — |
| deepseek/zhipu | ✅ 双 PASS | — |
| **provider 数** | **7 个**（含 gemini-3.6-flash！） | 新发现——v21 多了 gemini |
| 效率 | agent 烧满 15 步预算 | 功能对但浪费 |

---

## §2 v22 测试：四步

### Step 1：fmt 修复（✅ 已完成 2026-08-01，开发者执行）

```bash
cd ~/codex_work
cargo fmt --all            # ✅ 已执行，RC=0
cargo fmt --all -- --check # ✅ RC=0（v22 测试窗口可跳过本步直接 T2）
```

### Step 2：补跑 T6 基准快检（20 题 × 1 遍）

如果 T6 还没跑完：

```bash
cd ~/codex_work && source ~/.cargo/env
BENCH_PROVIDER=deepseek BENCH_OUT=v22-baseline.jsonl \
  python3 bench/runner.py batch --runs 1
```

| 期望 | 问题处理 |
|---|---|
| ≥ 17/20（85%） | ✅ 正常 |
| < 15/20 | ⚠️ 记录哪几题 FAIL |
| T19 仍 FAIL | 📝 记录（已知盲区） |
| T13 仍 FAIL | ⚠️ v20 修复后仍不稳——记录 |

### Step 3：补跑 T7 应力快检

```bash
cd ~/codex_work && source ~/.cargo/env
VM_PW=123456 CODEX_VM_PW=123456 STRESS_PROVIDER=deepseek STRESS_RUNS=3 \
  python3 bench/stress_v15.py
```

| 期望 | 问题处理 |
|---|---|
| ≥ 22/24 PASS | ✅ 正常 |
| 出现 panic 日志 | ❌ 立即记录场景和 `~/service.log` 中 panic 前后 20 行 |

### Step 4：Gemini provider 冒烟测试（新增项）

v21 测试发现 `/api/v1/models` 返回了 **gemini-3.6-flash**——这是一个未被文档记录的新 provider。必须验证它是否可用：

```bash
cd ~/codex_work && source ~/.cargo/env
# 用 gemini 跑最简单的冒烟任务
BENCH_PROVIDER=gemini BENCH_BUDGET=15 BENCH_TIMEOUT_MIN=10 \
  python3 bench/runner.py run T00-smoke --run 1
```

| 期望 | 问题处理 |
|---|---|
| `VERIFY_PASS` | ✅ gemini provider 可用，记入 provider 矩阵 |
| `FAIL` 但无崩溃 | ⚠️ gemini 能调用但能力有限，记录失败模式 |
| 连接失败 / 401 / timeout | ❌ gemini provider 注册了但不可达——检查 key 和端点 |

**如果 gemini 可用**：它是 provider 矩阵的第 4 个可用 provider（deepseek / zhipu / agnes / gemini）。值得跑一次 20×1 对比，但不急——留 Q3 季度体检。

---

## §3 效率诊断（开发者侧，已诊断完成 2026-08-01）

v21 测试发现所有任务 `phase=error、success=true`——agent 每次都烧满 15 步预算，即使做对了也不提前完成。

**✅ 诊断结论（已实测）**：

```
方法：录 T00 单题完整事件流（34 条消息 / 18 steps）
结果：phase=done, ok=true, status=completed —— 不是 error！
  工具调用序列（4 次，干净收敛）：
    glob → read src/lib.rs → write_file(multiply) → bash(cargo test) → Done

结论：
1. phase=error 是 v21 测试时的旧 release 二进制现象（VM 未随代码更新编译）
2. steps=18 是 phase 转换计数（Init→Plan→Act→Observe→Reflect 每轮 +3~4），
   不代表"多做事"——实际工具调用仅 4 次
3. v22 无需修 agent 效率——当前代码行为正常，不修改任何东西
```

不需要 v22 测试窗口做——已由开发者完成并结案。

---

## §4 测试报告模板（v22 追加）

在 v21 报告后面追加：

```
=== v22 深测追加 ===

T6 基准快检：
  PASS 数: ___ / 20
  FAIL 题: ___________
  比 v20 基线 87.5%：___ (上升 / 持平 / 下降)

T7 应力快检：
  PASS 数: ___ / 24
  panic: ___ (YES / NO)

Step 4 Gemini：
  T00-smoke: ___ (PASS / FAIL)
  失败模式: ___________ (如果是 FAIL)

新增问题：
  1. 
  2.
```

---

## §5 测试窗口排期

| 步骤 | 耗时 | 谁做 |
|---|---|---|
| Step 1 fmt | 30 秒 | 开发者 |
| Step 2 T6 基准 | 15 分钟机器 | 测试窗口 |
| Step 3 T7 应力 | 12 分钟机器 | 测试窗口 |
| Step 4 Gemini 冒烟 | 2 分钟 | 测试窗口 |
| Step 1 后重新 T1-T3 | 5 分钟 | 测试窗口（fmt 修复后验证） |
