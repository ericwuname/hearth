# v24-post 基准缺陷修复验收报告 — P1~P4 四件套

> 日期：2026-08-06 | 基线：`09d1271` → 验收后 `012d3ac`
> 依据：`bench-issues-tasklist-2026-08-06.md`（560-run 马拉松证据）+ `bench-fix-task-order-2026-08-06.md`
> 性质：基准 harness 正确化四件套——**P3/P4 闭环验证通过，P1 代码落地，P2 根因归因**
> ⚠️ 用户已入睡——本报告为自主执行 + 诚实记录，含一处验证通道限制（deepseek key 402）

---

## 〇、审查补充优化（用户问"有没有补充优化的地方"）

1. **P3 落点修正**：任务书给两个方向（runner prepend / sandbox 固化）——审查定版为
   **工具层注入**（tools-builtin/bash.rs env 构造时 prepend `$HOME/.cargo/bin`）：
   不碰 G0 sandbox crate（影响面最小）、不污染 goal 文本（runner prepend 会混进目标描述）。
2. **P3 边界补强**：原 PATH 缺失时保留系统默认路径（`/usr/local/bin:/usr/bin:/bin`）——
   否则 bash 自身都找不到（测试当场暴露：test_bash_echo/failure 全部 os error 2）。
3. **P4 复用已有端点**：transcript 落盘直接 GET `/events`（WP-2 的 JSONL 录制端点），
   不新增端点——runner 一行改造。
4. **P1 双重强化**：goal.txt 写死签名（局部）+ system prompt `IDENTIFIER CONTRACT`（全局，
   有条件的约束"当题目指定符号名时"——不改变其他任务行为）。
5. **验证环境关键发现**（transcript 红利）：service 启动目录决定 session workspace
   （`current_dir/sessions/{sid}`），必须与 runner 上传路径（`~/codex_work/sessions/{sid}`）
   一致——马拉松 service 从 `~/codex_work` 启动才匹配。本次从 `~/codex` 启动导致 agent
   空目录工作（第一轮 6 runs 全 budget exhausted 的根因）。

## 一、执行结果

| 项 | 状态 | 证据 |
|---|---|---|
| **P4** transcript 落盘 | ✅ **闭环** | 每 run 完整 JSONL 落盘（`bench/results/transcripts/{sid}.jsonl`）——含 goal/span/tool_call/error/reflection |
| **P3** cargo PATH | ✅ **闭环** | bash.rs 注入 + 240 tests 零回归；T00 transcript 佐证 agent 自测路径 |
| **P1** 命名硬约束 | ✅ 代码落地 / ⏸ LLM 验证挂账 | goal 签名写死 + IDENTIFIER CONTRACT 进 system prompt；编译 + 测试过 |
| **P2** T13 分析 | ✅ 归因完成 | transcript 揭示：验证轮失败 = key 402（非能力）；马拉松 37.5% = 真实能力边界 |

## 二、验证轮详情（诚实记录）

**第一轮（service 从 ~/codex 启动）**：6 runs 全 budget exhausted + **0 次 write_file**——
agent 在空目录工作（grep "(no matches)"、bash `find /` 找 Cargo.toml）。**根因 = workspace
目录不一致**（service `current_dir/sessions/{sid}` ≠ runner 上传 `~/codex_work/sessions/{sid}`）。

**第二轮（service 从 ~/codex_work 启动）**：6 runs 全 57s 一致 error——transcript 揭示
**真因**：`OpenAI error 402 Payment Required: Insufficient Balance`——**deepseek fallback key
余额不足**，agent 在 Plan 相位首次 LLM 调用即失败。**这不是 P1/P2 修复无效**，是验证通道没钱。

**P4 价值实证**：402 这类失败在 560-run 马拉松时代（零 transcript）根本无法定位——
本次 transcript 一读即中。这正是 P4 的最高杠杆意义。

## 三、门禁汇总（含 08-07 补验）

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **240 passed 零失败** |
| cargo build --release | ✅ 成功 |
| transcript 落盘 | ✅ 实证（T00 72 行 + 每 run JSONL） |
| P1/P2 LLM 验证（08-07，用户充值后） | ✅ 完成——见 §六 |

## 六、LLM 验证补充报告（2026-08-07，deepseek 充值后）

**执行条件**：deepseek fallback key 充值生效（probe 200）；**service 从 `~/codex_work` 启动**
（workspace 与 runner 上传路径一致——首轮误跑旧 service cwd=~/codex 已排除）。

| 任务 | 结果 | 与马拉松对比 | 结论 |
|---|---|---|---|
| **T13-fix-index** ×3 | ✅ **3/3 PASS** | 37.5% → **100%** | P2 实质改善（环境修正 + 能力） |
| **T19-merge-duplicate** ×3 | ❌ 0/3 | 0/8 → 0/3 | **模型能力墙**（见下） |

**T19 深度归因（P4 transcript 红利）**：
1. agent 行为 = `grep → read → 文本宣称完成`——**0 次 write_file**，`done ok=True`（虚假完成）。
   命名约束（P1 goal + IDENTIFIER CONTRACT）根本没机会执行——agent 不写代码。
2. **根治**：v22 gate 兜底（replan 3 次放行 Done）成为只读虚假完成通道 → 改为
   **read-only 永不 Done**（仅 read/grep/glob 且无 write → 注入 write-required 强提示 +
   继续 replan，budget 为真实边界）+ `acted` 判定（bash/write 算行动，避免误伤机制测试）。
3. gate 后 T19 ×3 = **诚实 budget_exhausted**（~600s/run，不再 fake done）——通过率仍 0，
   根因 = **deepseek-v4-flash 读完代码不产出 write_file**（模型能力墙，与 T19 命名无关）。
4. **修正马拉松结论**：T19 的 NO_GENERIC_FN ≠ 命名不遵从——是"只读即完成"虚假 done 的
   残留表现；P1 命名约束是必要修复（防止真实命名错误）但 T19 主因是能力墙。

**新增修复（commit `9b8d13b`）**：loop.rs read-only gate + acted 判定——240 tests 零回归。

## 四、Known Issues（环境，非任务缺陷）

| 项 | 说明 | 影响 |
|---|---|---|
| **VM landlock 退化** | `landlock not available in child (os error 42)`——sandbox 只读隔离失效（Skipping FS isolation），45+ 次 | 🔴 G0 关注：隔离失效需在 VM 恢复 landlock（内核/VMware 配置）后复验 |
| **workspace 目录一致性** | service `current_dir` 决定 workspace；runner 上传路径必须匹配 | 文档化：马拉松 service 必须从 `~/codex_work` 启动（本轮已实证——cwd 错误时 agent 空目录） |
| ~~deepseek key 402~~ | ~~马拉松 key 未留存，fallback key 余额不足~~ | ✅ 2026-08-07 用户充值后解除（probe 200 + 验证完成） |

## 五、P1 修复内容（代码层已闭环）

1. **T19 goal.txt**：写死 `pub fn parse_positive<T>(s: &str) -> Result<T, String> where T: FromStr + PartialOrd + Copy + Clone` + 禁止变体清单（parse_positive_num / parse_positive_u8 / positive_parse）。
2. **system prompt（loop.rs build_messages）**：新增 `IDENTIFIER CONTRACT`——"When the task specifies a symbol name, you MUST define exactly that identifier... The acceptance script greps the exact name; a different name is a FAILED task."

## 六、交付

- commit `012d3ac`（4 文件：bash.rs / loop.rs / runner.py / T19 goal.txt）
- 挂账：P1 效果验证（deepseek 20×1 + T19 3 次）——待预算 key；landlock 恢复后复验 sandbox
