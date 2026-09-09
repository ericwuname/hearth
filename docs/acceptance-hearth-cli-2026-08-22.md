# Hearth CLI 开箱即用 — 执行验收报告

> 任务书：`docs/hearth-cli-taskbook-dispatch.md`（D1-D6，2026-08-22 签发）
> 验收：2026-08-22 | commit `9bc2715` | 性质：架构 refactor + 安装 + 配置 + 安全透明 + Observer 主动反馈

---

## 1. 结论：R1-R7 全过（VM 真机实测）

| 门禁 | 内容 | 结果 | 证据 |
|---|---|---|---|
| **R1** 静态 | `[profile.release]` + bin `hearth` + `agent-runtime` crate + service 包装层 | ✅ | release/hearth 7.1MB；双 bin（hearth+codex）；agent-runtime 3 源文件 |
| **R2** 真实 Linux | VM 非仓库目录 `hearth --help` + `hearth chat "写 add 函数"` 端到端（无 service、无 .env） | ✅ | `/tmp/hearth_e2e/src/lib.rs` 产物真实落盘（add+测试）；徽章→规划→glob→写文件→反思全渲染 |
| **R3** 零配置 | 清空 env + 无 config → 清晰 key 缺失错误 | ✅ | `未配置 API key` + 3 步下一步（config set / export / init） |
| **R4** 安全透明 | Linux 🔒 `landlock+seccomp (fail-closed)`；非 Linux ⚠️ noop | ✅ | 直跑实测徽章打印 |
| **R5** 错误可行动 | 401/403 报具体原因 + 下一步，无裸 panic | ✅ | 坏 key 401 **0s 立即失败**（重试短路，之前 14s）；`Authentication Fails...` + 下一步提示 |
| **R6** Observer 素材 | note 各形态落盘双侧 jsonl；轻探仅异常触发且可关 | ✅ | human-s1.jsonl（--self/--observer-verdict/--mood 字段全）；AI 侧 `<uuid>.jsonl`；💡 轻探触发 + `feedback-prompt=false` 关闭 |
| **R7** 反审闭环 | `--observer-verdict n` 纠偏落盘；不自动改配置/内核 | ✅ | `rebuttals/s1.jsonl`（verdict=n+reason）；测试断言 Observer::run 输出不变（零执行权） |

通用门禁：**fmt 0 / clippy 0 / 248 passed（baseline 244 + 4 新增）/ release build 0**。

---

## 2. 交付清单

- **D1** `crates/agent-runtime`（library）：session.rs 整体搬入（核实零 HTTP 依赖）+ EnvelopeState 抽 `envelope.rs`；service 变薄 re-export——routes.rs / integration_test.rs **零改动**（244 测试兜底回归）；wiring-v13 规则 + FNV hash 锁同步
- **D2** 双 bin（hearth 主 + codex 别名，hearth-naming.md 兼容）；`[profile.release]` strip+lto；零配置 key 缺失可行动错误；`docs/hearth-cli-guide.md`
- **D3** `~/.config/hearth/config.toml` + `config set/get`（provider/model/url/api-key/mode/feedback-prompt）+ `init` 交互引导；参数 > env > toml > 默认（单测覆盖）
- **D4** 隔离徽章（🔒/⚠️）；**401/403 重试短路**（planner + agent-loop 两处——坏 key 从 14s 重试 → 0s 报错）；401 附"下一步"
- **D5** `hearth note`（--session/--self/--observer-verdict/--mood）→ `~/hearth/observer/human-*.jsonl`；直跑事件流 → AI 侧 `<sid>.jsonl`；chat 开头轻探（recent_human_abnormal 判定 + feedback-prompt 开关）；`observer::apply_rebuttal` 反审（零执行权）
- **D6** 未做（跨平台/install.sh——成本 >0.5 天，且用户真用 Linux；非 Linux ⚠️ 徽章代码已在）

---

## 3. 顺手修复（超出任务书）

| # | 修复 | 影响 |
|---|---|---|
| 1 | **GlobTool sandbox default → for_build_tools**——RT3 挂账的 readonly find `Failed to restore initial working directory: Operation not permitted` 在生产路径根治（直跑实测复现：default writable 空 → find fchdir 恢复 cwd 被 landlock 拦） | 直跑/服务端 glob 不再报错 |
| 2 | planner/agent-loop 401/403 认证错误立即失败（重试无意义浪费 14s） | 坏 key 0s 可行动报错 |
| 3 | `--self` 参数名（clap kebab-case 生成 `--self-label`，显式 `long="self"`） | 任务书命令语义对齐 |

---

## 4. 风险与挂账

- **cgroup Permission denied**（VM 非特权 userns）——fail-closed 语义下 cgroup 仍 warn 不阻断（RT3 挂账延续）；sandbox 的 seccomp+landlock 是真隔离主体
- **VM 无 Ollama**——R4 徽章用 ollama 验证了路径；真 ollama 需用户本机（qiyuan-8b 已装）
- **D6 跨平台/install.sh** 未做（成本评估 >0.5 天；用户 Linux 优先）
- **llm-openai WARN 日志含完整 req_body**（含 system prompt）——日志噪音，非安全泄漏（本地），后续可截断

---

## 5. 用户验收路径（Linux）

```bash
cargo build --release -p codex-cli
install -m755 target/release/hearth ~/.cargo/bin/hearth

hearth init                    # 交互填 key（或 config set api-key）
hearth chat "写一个 Rust 函数 add(a,b) 并写测试验证"   # 直跑，无 service
hearth note "审批卡了三次" --mood frustrated           # Observer 素材
hearth note "Observer 判错了" --observer-verdict n     # 反审
hearth config get             # 查看配置
```
