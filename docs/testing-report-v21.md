# v21 全链路测试报告（执行窗口实测）

> 执行方：测试窗口（AI 在 VM `192.168.220.131` 上照 `docs/testing-runbook-v21.md` 实测）
> 执行时间：2026-08-01
> 代码版本：VM `~/codex_work`（release 二进制，wiring 14/14 → 确认 v21 级代码）
> 原则：**不改任何代码，只跑测试、整理报告**。runbook 与 VM 实际环境的 9 处差异见文末「runbook 适配说明」，均按"记录不修复"处理。

---

## 结果模板（实测填写）

```
=== v21 全链路测试报告 ===

T1 编译门：
  fmt:    FAIL (RC=1, 8 处 Diff：loop.rs / llm-replay/src/lib.rs / service/src/main.rs)
  clippy: PASS (0 warning)
  test:   PASS (205 passed / 0 failed)
  wiring: PASS (14/14，0 broken —— 确认 v21)

T2 服务冒烟：
  启动:  OK (release 实例 pid 3310534，/healthz=200)
  T00:   PASS (deepseek，VERIFY_PASS，panic=0)

T3 接线自证：
  断开后变红: YES (13/14，tool-exchange 断言 broken —— 防火墙是活的)
  恢复后变绿: YES (14/14 恢复)

T4 经验持久化：
  重启前经验数: 88
  重启后经验数: 88
  持久化: OK (重启不丢)

T5 provider：
  deepseek: PASS (T01-read-api，VERIFY_PASS)
  zhipu:    PASS (T02-change-timeout，VERIFY_PASS)

T6 基准快检：
  通过数: __ / 20   （待 T6 跑完填写）
  FAIL 题: ___________ （待填）

T7 应力快检：
  PASS 数: __ / 24   （待 T7 跑完填写）
  有无 panic: ___    （待填）

总结问题：
  1. T1 fmt 不干净（8 处 diff，v21 清密钥编辑遗留的格式漂移）—— 记录给开发者修复
  2. runbook 与 VM 实际环境有 9 处命令/路径/参数差异（见文末），按"记录不修复"处理
  3. 所有任务 session 均 phase=error 但 success=true（agent 烧满 15 步预算、done 检测偏保守）
     —— 不影响 VERIFY_PASS 判定，但提示效率可优化
```

---

## 逐测试证据

### T1 编译门（实测）
- T1.1 `cargo fmt --all -- --check` → **RC=1 FAIL**。8 处 diff，落在：`crates/agent-core/src/loop.rs`、`crates/llm-replay/src/lib.rs`、`crates/service/src/main.rs`（v21 清密钥编辑遗留的格式漂移）。
- T1.2 `cargo clippy --workspace --all-targets -- -D warnings` → **PASS**（0 warning / 0 error）。
- T1.3 `cargo test --all` → **PASS**（205 passed / 0 failed）。
- T1.4 `cargo run -p project-xray -- wiring --spec docs/xray/wiring-v13.toml` → **PASS**（`14/14 pass, 0 broken`，含 3 条 v21 新增能力断言：experience-adaptive-switch / all-done-requires-write / experience-prune-wired → 确认代码为 v21 级）。

### T2 服务启动与冒烟
- `fuser -k 3000/tcp` 清掉旧占用进程 → 启 release 实例（pid 3310534），`/healthz`=200 → **OK**。
- T00-smoke（`BENCH_PROVIDER=deepseek`）：`"success": true`，verify_tail `VERIFY_PASS`，panic=0 → **PASS**。

### T3 接线自证
- 备份 `loop.rs` → 将 `self.record_tool_exchange();` 调用**整行替换**为禁用字面量（注意：runbook 的"注释法"会让 wiring 的 `contains()` 子串仍命中 → 假绿，故改用整体替换触发真红）。
- 重跑 wiring → **13/14（1 broken，tool-exchange 断言变红）** → 防火墙是活的。
- 恢复备份（md5 校验一致）→ 重跑 → **14/14 恢复绿** → **YES/YES**。

### T4 经验持久化
- 重启前 `total_experiences = 88`；重启服务后（释放 old pid、重启 release 实例）再查 = `88` → **OK，持久化生效**。

### T5 provider 连接性
- `/api/v1/models` 返回 7 个 provider：gemini-3.6-flash / glm-4.5-air(zhipu) / glm-4.7(zhipu-max) / agnes-2.5-flash / deepseek-v4-flash / gpt-4o / deepseek-v4-pro。
- deepseek T01-read-api → `"success": true`，`VERIFY_PASS` → **PASS**。
- zhipu T02-change-timeout → `"success": true`，`VERIFY_PASS` → **PASS**。
- 两者均 `phase=error` 但 `success=true`（烧满 15 步预算）。

### T6 基准快检（20 题 × 1 遍，deepseek 标准脑）
- 命令：`cd ~/codex_work && source ~/.cargo/env && CODEX_VM_PW=123456 BENCH_PROVIDER=deepseek BENCH_OUT=v21-baseline.jsonl python3 bench/runner.py batch --runs 1`
- 结果：**待填写**（期望 ≥ 17/20）。

### T7 应力快检（8 场景 × 3 = 24 次）
- 命令：`cd ~/codex_work && source ~/.cargo/env && VM_PW=123456 CODEX_VM_PW=123456 STRESS_PROVIDER=deepseek STRESS_RUNS=3 python3 bench/stress_v15.py`
- 结果：**待填写**（期望 ≥ 22/24 PASS，无 panic）。
- 注意：`stress_v15.py` 的 `service_alive()` 用 `target/debug/service` 模式匹配，而线上服务是 **release** 二进制 → 该字段恒为 False（harness 命名错位，非服务真死）。判定以 `ok` 与 `no_panic` 为准；panic 检测读 `~/service.log`（已建软链到真实 release 日志）。

---

## runbook 适配说明（9 处差异，均"记录不修复"）

| # | runbook 写法 | VM 实际 | 处理 |
|---|---|---|---|
| 1 | `cd ~/codex_work/codex-rust-v1.0-final` | 代码根即 `~/codex_work` | 全部改为 `cd ~/codex_work` |
| 2 | T2/T4 `pkill -f "codex-rust-v1"` | 进程名是 `target/release/service` | 改用 `fuser -k 3000/tcp` + `pkill -f target/release/service` |
| 3 | T2/T4 `nohup cargo run -p service --release` | 用预编译 `./target/release/service` 分离启动 | 避免重复编译，端口不冲突 |
| 4 | T3 `sed` 注释 `record_tool_exchange` | wiring 用 `content.contains()` 子串，注释仍命中 → 假绿 | 整行替换字面量触发真红，恢复后 md5 校验 |
| 5 | T5.3 `V17_PROVIDER=zhipu` | 实际环境变量 `BENCH_PROVIDER` | 改用 `BENCH_PROVIDER=zhipu` |
| 6 | T6 `batch --tasks all --runs 1` | `--tasks all` 过滤成空跑 | 改为 `batch --runs 1`（省略 --tasks） |
| 7 | T6.2 `grep -c VERIFY_PASS bench/results/raw/*.jsonl` | 多历史文件会重复计数 | 按本次 `BENCH_OUT` 单文件计数 |
| 8 | T7 日志路径 `~/codex_work/logs/service.log` | 实际日志 `~/service.log` | 启动服务时 `ln -sf ~/service.log ~/codex_work/logs/service.log` |
| 9 | `stress_v15.py` `service_alive` 匹配 `target/debug/service` | 线上是 release 二进制 | 字段恒 False（无害），判定看 `ok`/`no_panic` |

> 另：VM 缺 `git`（tar 上传），版本靠 `wiring 14/14` + `.env` 时间标记推断为 v21 级；VM `python3` 受 PEP 668 限制，runner 所需 `paramiko` 用 `pip install --user --break-system-packages` 装到 `~/.local`。

---

## 总判定

- **可放行项**：T1(clippy/test/wiring)、T2、T3、T4、T5 全 PASS；T6/T7 待填。
- **需开发者跟进**：
  1. **T1 fmt 不干净**（8 处 diff）—— 按 runbook「记录不修复」原则上报，建议 `cargo fmt` 统一格式。
  2. **runbook 9 处命令差异** —— 建议同步修订测试手册，避免后续测试窗口踩坑。
  3. **agent 效率**：所有任务 `phase=error` 但 `success=true`（烧满 15 步预算、done 检测偏保守）—— 功能正确，但步数预算可优化。
