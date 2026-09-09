# codex-rust v21 全链路测试手册

> 给测试窗口。不需要懂 Rust、不需要懂项目架构。
> 每步都有：复制粘贴的命令 → 期望看到什么 → 看到不对怎么办。
> 总耗时约 60 分钟（含机器跑分时间）。

---

## 测试环境

```
VM 地址：ssh wutao@192.168.220.131
VM 密码：123456
代码位置：~/codex_work
服务端口：3000
DeepSeek API 余额：~¥7（充足）
每任务成本：~¥0.01
```

---

## 测试一：编译门（5 分钟）

**目的**：确认代码能不能编译、格式对不对、有没有编译警告。

### T1.1 代码格式

```bash
ssh wutao@192.168.220.131
cd ~/codex_work/codex-rust-v1.0-final
source ~/.cargo/env
cargo fmt --all -- --check
```

| 期望 | 问题处理 |
|---|---|
| 没有任何输出（或只输出空白） | ✅ 通过 |
| 出现 `Diff in ...` | ❌ 格式不干净，记录文件名告诉开发者 |

### T1.2 编译警告检查

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

| 期望 | 问题处理 |
|---|---|
| 最终输出不含 `warning:` 或 `error:` | ✅ 通过 |
| 出现 `warning:` | ❌ 记录 warning 内容 |

### T1.3 全部测试

```bash
cargo test --all
```

| 期望 | 问题处理 |
|---|---|
| `test result: ok.` 全部 PASS，最终 `0 failed` | ✅ 通过 |
| 出现 `FAILED` | ❌ 记录失败测试名称 |

### T1.4 接线断言

```bash
cargo run -p project-xray -- wiring --spec docs/xray/wiring-v13.toml
```

| 期望 | 问题处理 |
|---|---|
| `14/14 pass, 0 broken` | ✅ 通过 |
| 出现 `broken` 或 `RED BREAK` | ❌ 记录断裂的断言 ID，报告开发者 |

---

## 测试二：服务启动与冒烟（10 分钟）

**目的**：确认服务能启动、能接收请求、能跑完一个最简单的任务。

### T2.1 启动服务

```bash
# 先杀掉旧的服务（如果有的话）
pkill -f "codex-rust-v1" 2>/dev/null; sleep 2

# 启动服务（后台运行，日志写文件）
mkdir -p ~/codex_work/logs
cd ~/codex_work/codex-rust-v1.0-final
nohup cargo run -p service --release > ~/codex_work/logs/service.log 2>&1 &
echo "服务启动中，等待 30 秒..."
sleep 30
```

注意：首次 `--release` 编译约 2-3 分钟（增量编译 < 30 秒）。

### T2.2 健康检查

```bash
curl -s http://localhost:3000/healthz
```

| 期望 | 问题处理 |
|---|---|
| 返回 `OK` 或 `200` | ✅ 通过 |
| 无响应 / `connection refused` | ❌ 等 30 秒重试。还不行 → 看日志 `tail -50 ~/codex_work/logs/service.log` |

### T2.3 冒烟任务 T00（最简单的任务：加一个函数）

```bash
cd ~/codex_work/codex-rust-v1.0-final
python3 bench/runner.py run T00-smoke --run 1
```

| 期望 | 问题处理 |
|---|---|
| 输出 `VERIFY_PASS` | ✅ 端到端全链路通 |
| 输出 `FAIL` | ❌ 记录失败信息 |

### T2.4 冒烟后检查服务日志

```bash
tail -20 ~/codex_work/logs/service.log
```

检查是否有 `panic` / `error` / `failed` 关键词。没有就 ✅。

---

## 测试三：接线自证（5 分钟）

**目的**：确认接线防火墙是真的——断开一条线能变红，恢复能变绿。

### T3.1 创建备份并断开一条关键接线

```bash
cd ~/codex_work/codex-rust-v1.0-final
# 备份原始文件
cp crates/agent-core/src/loop.rs crates/agent-core/src/loop.rs.backup

# 注释掉 record_tool_exchange 调用（断开 tool 回写这条关键接线）
sed -i 's/self\.record_tool_exchange();/\/\/ self.record_tool_exchange();/' crates/agent-core/src/loop.rs
```

### T3.2 验证接线变红

```bash
cargo run -p project-xray -- wiring --spec docs/xray/wiring-v13.toml
```

| 期望 | 问题处理 |
|---|---|
| 出现 `1 broken` 且包含 `tool-exchange-wired` | ✅ 接线防火墙是活的 |
| 仍然是 `14/14` 全绿 | ❌ 防火墙失效——报告开发者 |

### T3.3 恢复并验证变绿

```bash
cp crates/agent-core/src/loop.rs.backup crates/agent-core/src/loop.rs
cargo run -p project-xray -- wiring --spec docs/xray/wiring-v13.toml
```

期望：恢复成 `14/14 pass, 0 broken`。✅。

---

## 测试四：经验持久化（5 分钟）

**目的**：确认经验在服务重启后不丢失。

### T4.1 查看当前经验数

```bash
curl -s http://localhost:3000/api/v1/experience/metrics | python3 -m json.tool | grep total_experiences
```

记录数字（比如 `"total_experiences": 5`）。

### T4.2 重启服务

```bash
pkill -f "codex-rust-v1" 2>/dev/null; sleep 3
cd ~/codex_work/codex-rust-v1.0-final
nohup cargo run -p service --release > ~/codex_work/logs/service.log 2>&1 &
sleep 30
```

### T4.3 再次查看经验数

```bash
curl -s http://localhost:3000/api/v1/experience/metrics | python3 -m json.tool | grep total_experiences
```

| 期望 | 问题处理 |
|---|---|
| 数字 ≥ T4.1 记录的数字（或至少 > 0） | ✅ 持久化生效 |
| 数字 = 0 且 T4.1 时 > 0 | ❌ 经验数据丢失 |

---

## 测试五：provider 连接性（10 分钟）

**目的**：确认每个配置的 LLM provider 都能正常处理请求。

### T5.1 列出可用模型

```bash
curl -s http://localhost:3000/api/v1/models | python3 -m json.tool
```

期望输出里能看到 `deepseek`、`zhipu` 等模型名。记录有哪些模型。

### T5.2 用 deepseek 跑冒烟

```bash
cd ~/codex_work/codex-rust-v1.0-final
python3 bench/runner.py run T01-read-api --run 1
```

| 期望 | 问题处理 |
|---|---|
| `VERIFY_PASS` | ✅ |
| `FAIL` | ❌ 记录失败原因 |

### T5.3 用 zhipu 跑冒烟（如果配置了）

```bash
V17_PROVIDER=zhipu python3 bench/runner.py run T02-change-timeout --run 1
```

同样期望 `VERIFY_PASS`。

---

## 测试六：基准快检（15 分钟，机器时间）

**目的**：快速跑一遍 20 题 × 1 遍，拿到当前通过率快照。

### T6.1 跑快检基准

```bash
cd ~/codex_work/codex-rust-v1.0-final
python3 bench/runner.py batch --tasks all --runs 1
```

预计耗时约 15 分钟（20 题 × 平均 45 秒）。

### T6.2 看结果

```bash
# 数下 PASS 了几个
grep -c "VERIFY_PASS" bench/results/raw/*.jsonl 2>/dev/null | tail -1
# 或者直接看最后的汇总输出
```

记录：PASS 数 / 20。

| 期望 | 问题处理 |
|---|---|
| ≥ 17/20（85%） | ✅ 正常波动 |
| < 17/20 | ⚠️ 记录哪几题 FAIL |

---

## 测试七：应力快检（15 分钟，机器时间）

**目的**：确认极端场景下服务不崩溃。

### T7.1 跑应力测试

```bash
cd ~/codex_work/codex-rust-v1.0-final
python3 bench/stress_v15.py
```

预计耗时约 12 分钟（8 场景 × 1-2 分钟）。

### T7.2 看结果

| 期望 | 问题处理 |
|---|---|
| 输出含 `no_panic` 且无 panic 报告 | ✅ |
| 24 次运行 ≥ 22 次 PASS | ✅ |
| 出现 panic 或服务进程死掉 | ❌ 记录哪个场景、什么现象 |

---

## 测试结果模板

全部跑完后，填这张表发给开发者：

```
=== v21 全链路测试报告 ===

T1 编译门：
  fmt:    ___ (PASS / FAIL)
  clippy: ___ (PASS / FAIL)  
  test:   ___ (PASS / FAIL——__ passed / __ failed)
  wiring: ___ (PASS / FAIL——__/14)

T2 服务冒烟：
  启动:  ___ (OK / FAIL)
  T00:   ___ (PASS / FAIL)

T3 接线自证：
  断开后变红: ___ (YES / NO——防火墙失效)
  恢复后变绿: ___ (YES / NO)

T4 经验持久化：
  重启前经验数: ___
  重启后经验数: ___
  持久化: ___ (OK / FAIL)

T5 provider：
  deepseek: ___ (PASS / FAIL)
  zhipu:    ___ (PASS / FAIL)

T6 基准快检：
  通过数: ___ / 20
  FAIL 题: ___________

T7 应力快检：
  PASS 数: ___ / 24
  有无 panic: ___

总结问题：
  1. 
  2. 
  3.
```

---

*本手册设计原则：每步都可以复制粘贴。每步都有明确的"什么算过、什么算没过"。不需要理解代码，只需要看输出、填表、传过来。*
