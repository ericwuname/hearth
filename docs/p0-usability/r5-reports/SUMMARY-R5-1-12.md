# R5 智能性根治 · 12 件施工战报汇总（traecode 执行窗 → 砺/顶层）

> **分支**：`p0-usability-01`　**施工窗**：2026-09-06　**基线**：0.2.24（5c24e1a，门禁 501/0）
> **完工版本**：**0.2.25**（a4a2f91）　**逐件 commit 清单见 §三**
> **验收纪律执行情况**：每件独立 commit（B.1 硬规则 1）、判据原文粘贴（A.1-3）、
> 改前必红实证（关键件 VM .131 独立复跑旧生产变体）、门禁原文粘贴（A.1-4）。

---

## 一、逐件战报（A.2 模板）

### [R5-1] 事实注入（包A·通路层）
- **病理**：observe_verdict / last_failure_class / last_recovery_strategy 生产端齐备（Observe/Reflect 每轮写 scratch），消费端止步于测试断言——策略算出来没回到决策者。
- **改动**：`5dd93ee` terminal.rs +37/-0（strategy_suggestion 纯函数+单测）；loop.rs +~70（last_observe_errored 新鲜度门 + build_messages L4 注入块 + 2 测试）。后随 `e7972cb` fmt 规整。
- **判据**：C 语料 L285 同错重复 3→≤2 且第二次命令不同 → **机制通路单测锁定**（test_r51_failure_facts_injected_into_next_messages：class/strategy/suggestion 三要素注入断言），**真机计数归跑测阶段**（本窗 .133 曾失联，恢复后已可用，但 C 语料重放属总验收跑测项，见 §四）。
- **门禁**：FMT=0 CLIPPY=0 TEST=0 PASSED=507（501+6）
- **自检**：新鲜度门用 observe_verdict.has_error（每轮覆写）防陈旧注入；strategy/suggestion 经注入块同轮到达而非内联 ERROR 行（分类在 Reflect 相位才产出——时序诚实约束，与委托书字面格式的偏差已注明）。

### [R5-2] 错误回喂结构化（包A）
- **病理**：裸 `ERROR: {raw}` 只有原始输出，恢复引导缺位。
- **改动**：`fd28ba2` loop.rs +11/-1——`ERROR[class={error_kind:?}]: {raw}`（error_kind 来自调度层 downcast，RC20 纪律禁文本解析）。
- **判据**：同 R5-1；test_r52_error_line_structured_with_class（ExitNonZero(1) → 前缀断言）。
- **改前必红**：✅ VM .131 实测旧生产变体 3 正例红（R5-1×1 + R5-2×1 + R5-3×1 组内）。
- **门禁**：同上。
- **自检**：与委托书字面格式（ERROR[class strategy suggestion]）的偏差：strategy/suggestion 无法在记录时点诚实产出（分类在 Reflect 相位），改由 R5-1 注入块同轮送达——**意图满足（恢复引导随错误到达决策者），字面偏差如实声明**。

### [R5-3] 强制换策略（包A）
- **病理**：same_tool_repeat ≥2 判 PlanFailure→Replan 但只写 scratch 遥测——同款调用原样重试，"同错重复 3 次"机制根源。
- **改动**：`a8fefbb` loop.rs +21——Replan && repeat≥2 → needs_decompose 置位 + `[strategy-forced]` 换法指令入史。
- **判据**：全语料"同错重复 3 次"=0 → 机制锁定（test_r53_same_tool_repeat_forces_strategy_change + 反例），真机计数归跑测。
- **改前必红**：✅ 实证（旧生产变体红）。
- **门禁**：同上。
- **自检**：无界循环由 budget max_steps 硬上限兜底（强制重分解同样消耗 steps）；verdict 仍归 planner（不越权强改判定，replan cap 语义保持）。

### [R5-4] 环境上下文注入（包B）
- **病理**：system prompt 零环境注入 → 首轮瞎猜技术栈（"hearth TUI 项目"搜 *.go）。
- **改动**：`1c8c33a` loop.rs +151——load_env_context()（cwd/技术栈 marker 确定性检测/git 分支/顶层文件树≤20；构造时计算一次，load_hearth_md 同款模式，全 best-effort）+ L2 注入。
- **判据**：用户原话场景单次可判负（首轮不得搜 *.go）→ 机制锁定（test_r54_env_context_injected_into_system_prompt：Rust 判定+cwd+文件树断言），**真机单次判负归总验收跑测**。
- **改前必红**：✅ 实证（旧生产 2/2 红 → 新 2/2 绿）。
- **门禁**：FMT=0 CLIPPY=0 TEST=0。
- **自检**：git 用一次同步 spawn（构造期 ~10ms，非热路径，与 hearth_md 同款先例）；技术栈用 marker 文件判定（零猜测），无 marker 不冒充。

### [R5-5] read 分页+行号（包B）
- **病理**：read 整文件直出、无行号；82k 字符截断死结（引擎亲口承认），中段丢弃无提示。
- **改动**：`9d00f18` read.rs 重写 execute（offset/limit 1-based、cat -n 格式 `{:>6}\t`、默认 cap 2000 行、截断必带续读 offset 指引、EOF 标注、越界空窗口提示）+ loop.rs prompt read 签名同步。truncate_tool_output 6000/4000/1500 保持（read 层行级截断先行后只剩兜底作用）。
- **判据**：大文件分页可读且带行号；截断提示含"如何读剩余部分" → 单测 4 条锁定。
- **改前必红**：✅ 实证（旧生产 5/5 红 → 新 7/7 绿）。
- **门禁**：tools-builtin 64/64 → 65/65（含 r59）。
- **自检**：grep 检查无其他消费方依赖 read 输出格式；apply_patch 锚点从 read 行号输出复制更稳（描述里已写明）。

### [R5-6] archive 读回通道（包B）
- **病理**：R2-2 落盘后读侧全仓为 0——事实换个地方销毁；切片提示只给 grep 路径。
- **改动**：`498ba31` context.rs +52（archive_digest：JSONL→turn 索引+首条用户消息摘要，按 index 去重，best-effort None）+ loop.rs 切片提示接线（前 8 轮清单）。
- **判据**：跨 10 轮问"还记得早期事实吗"可答 → 机制锁定（test_r56_slice_note_includes_archive_digest：50 消息硬切片 → EARLY-FACT turn#100 经 digest 可见；test_r56_archive_digest_reads_back_and_dedups：读回+去重+缺文件 None）。
- **改前必红**：✅ 实证（旧生产接线 1 红/1 绿）。
- **门禁**：agent-core 内联绿。
- **自检**：digest 每轮切片时读小文件（best-effort，≤8 条有界）。

### [R5-8] verification 看退出码（包C）
- **病理**：is_verification_command 只看命令名不看退出码——失败命令拿 VERIFIED 的合法性来源（R4.1 半修后仍开放）。
- **改动**：`66eff77` loop.rs record_tool_exchange——calls/results 按索引配对，is_error=true（非零退出码）→ 不点亮 + warn；结果缺失保守不点亮；成功路径保持 C-1 语义。
- **判据**：构造"失败命令+VERIFIED"场景必红 → test_r58_failed_verification_command_does_not_light_verified（bash "exit 1" + ExitNonZero(1)）。
- **改前必红**：✅ 实证（旧生产 1 红/1 绿）。
- **门禁**：绿。
- **自检**：acceptance 路径（:5593）本就核验 exit code，不属本刀；`bash -c "cargo test" || true` 复合命令退出 0 → is_error=false → 点亮——语义正确（整体命令确实成功）。

### [R5-9] 工具描述五要素（包C）
- **病理**：read 描述 5 个词、glob/grep 一句话（根因四：工具契约贫瘠）。
- **改动**：`2bc6f37` 六工具（read/bash/write_file/apply_patch/glob/grep）描述重写，各含何时用/何时不用/示例/边界/错误解读；web_fetch 与 introspect 原有描述已达标不动。lib.rs +test_r59（六工具×5 要素断言）。
- **判据**：五要素齐 + 门禁全绿 ✅（tools-builtin 65 passed）。
- **改前必红**：旧描述为单句无要素标记，断言结构性全红（字符串必然性，未走 VM 变体——诚实声明）。
- **门禁**：tools-builtin 65/64+1 绿。
- **自检**：描述内容取自真实护栏语义（路径防护/3000 字符分批/唯一锚点/出网白名单/exit code），非模板填空。

### [R5-10] 死流程→原则（包C）
- **病理**："follow strictly, in order"/"NEVER call grep/glob more than once"/"MUST end by calling write_file" 三条机械脚本压力（根因三，作者自认注释在位）。
- **改动**：`ca8024f` loop.rs product 分支 prompt 整体重写为决策原则（"a loop, not a script — adapt as evidence arrives"；空搜索=证据；真改才推进）；安全边界全保留（IDENTIFIER CONTRACT/红构建不得收工/3000 字符分批）。
- **判据**：既有回归全绿（门禁含 R1/R2 全部门）；问答类不压写盘 → test_r510 两断言组。
- **改前必红**：旧 prompt 死指令在位 → 断言必红（字符串必然性）。
- **门禁**：绿。
- **自检**：QA 分支（R1 收尾既有）未动；R5-5 read 新签名已同步进工具清单。

### [R5-11] 纯回答型完成（包C）
- **病理**：完成判定 keyed to 写盘——C 语料 52 次决策行全带"本轮写盘 0 个"，问进度/诊断类被写盘计数绑架烧相位。
- **改动**：`33ea38d` loop.rs do_reflect T5 判定推广：`pending_results.is_empty()` → `written_files.is_empty()`——QA/诊断类零写盘合法（诊断跑过命令也 Continue），不再烧 reflect LLM。
- **判据**：疑问句/诊断类零写盘 completed 路径存在（E2E 锁定 test_r511_qa_goal_completes_zero_write：ok=true + files_changed 空）且 G-C 门不回退（R1-1 三臂测试族全绿）。
- **改前必红**：✅ 实证（旧生产变体 1 红/1 绿）。
- **门禁**：绿。
- **自检**：写过盘的 QA（异常形态）仍走 planner.reflect；R1-1 改判恒 UNVERIFIED+不计成功率口径零接触。

### [R5-12] 宪法配执行点（包C）
- **病理**：八条宪法只有注入没有机制化执行位。
- **改动**：`92f74c0` docs/constitution-execution-map.md（条文↔执行位↔触发↔抽查逐行映射；未落执行位条文显式列出）+ test_r512 接线断言（wiring assertion 模式，沿用 v13 constitution-reads-file 先例）。
- **判据**：映射表落盘（门禁锁存续）+ 可抽查触发（每行机器断言或日志锚点）✅。
- **改前必红**：映射表/断言测试为新出生件（红=不出生，结构性）。
- **门禁**：绿。
- **自检**：第 1/5 条为任务书指定两条，已配齐；第 2/3/4/6/7/8 条部分语义由既有机制间接承载——表内显式声明"不冒充已配齐"。

### [R5-7] temperature 分档（包C·依赖图末位）
- **病理**：主循环决策调用 0.5 高温——85 次重新规划的随机性燃料（goal-drift :1448 早已 0.0，同仓口径不一）。
- **改动**：`c02ccc0` loop.rs :3550 0.5 → 0.2（Plan+Act 合一调用，落两档交点）；max_tokens 8192 审视=保持（12004 字符截断病理证据在案）；bridge 0.7/0.8/0.5 核对=保留（多模型议事多样性语义，非 agent 决策路径）。
- **判据**：test_r57（源级断言 0.2 在位/0.5 清零/0.0 参照保持；concat! 拆串防自引用假红）。**T-1 决策门 A/B 三指标（规划次数/工具出错/Plan 耗时）归真机跑测**——预注册通过线见任务包 §五。
- **改前必红**：旧代码 0.5 在位 → 断言必红。
- **门禁**：绿。
- **自检**：单 LLM 架构下 Plan/Act 无法分档调用——取交点 0.2 是架构约束下的最优落位；T-1 数据若显示仍空转，六相收敛立项时再拆。

## 二、门禁读数（A.1-4）

| 时点 | FMT | CLIPPY | TEST | PASSED | 勾稽 |
|---|---|---|---|---|---|
| 基线（0.2.24=5c24e1a） | 0 | 0 | 0 | 501 | 委托书口径 ✅ 实测复验 |
| 包A 三件后 | 0 | 0 | 0 | 507 | 501+6（r51×2+r52×1+r53×2+r51-terminal×1 = 6）✅ |
| **最终（0.2.25 全量树）** | **0** | **0** | **0** | **524** | 501+23（r51×3+r52×1+r53×2+r54×2+r55×4+r56×2+r57×1+r58×2+r59×1+r510×2+r511×2+r512×1）✅ 双向验算成立 |

最终门禁原文（.131 `~/fa/p0_gate_r1full.log`，2026-09-06）：

```
FMT_RC=0
CLIPPY_RC=0
TEST_RC=0
524          ← grep -oE "ok. [0-9]+ passed" | 累加
0            ← grep -c "test result: FAILED"
```

## 三、commit 清单（每件一个独立 commit）

| 件 | commit | 文件 |
|---|---|---|
| R5-1 | `5dd93ee` | terminal.rs, loop.rs |
| R5-2 | `fd28ba2` | loop.rs |
| R5-3 | `a8fefbb` | loop.rs |
| （fmt） | `e7972cb` | loop.rs, terminal.rs |
| R5-4 | `1c8c33a` | loop.rs |
| R5-5 | `9d00f18` | read.rs, loop.rs |
| R5-6 | `498ba31` | context.rs, loop.rs |
| R5-8 | `66eff77` | loop.rs |
| R5-9 | `2bc6f37` | read/bash/edit/glob/grep/patch.rs, lib.rs |
| R5-10 | `ca8024f` | loop.rs |
| R5-11 | `33ea38d` | loop.rs |
| R5-12 | `92f74c0` | loop.rs, docs/constitution-execution-map.md |
| R5-7 | `c02ccc0` | loop.rs |
| 版本 | `a4a2f91` | Cargo.toml → 0.2.25 |

## 四、遗留与例外（即时上报项）

1. **VM 失联窗口**：施工开始时 .131/.133 均失联（~02:40-03:10 恢复）——期间完成源码精读与设计，恢复后全部在 VM 实证。
2. **本地工具链缺失**：Windows 侧无 cargo（rustup+GNU+MinGW 折腾后仍缺 dlltool/libgcc_eh 链路，且 MSVC 安装需提权被拒）——**编译验证全部改走 .131 VM**（cargo check 6.7s 增量，实际比本地更权威）。
3. **最终门禁**：FMT=0 / CLIPPY=0 / TEST=0 / **524 passed / 0 failed**（501+23 新测试，双向验算成立；原文见 §二）。
4. **清欠三件**：Q-3 ✅（重审计：旧首词法 13→新口径 4 个零验证 completed，9 个 run 平反）；Q-1 ✅（收尾三行 REPL 前台渲染实证）；Q-2 ✅（103% 未被引用，R5-5 行号真机可见）——详见 Q-clearance.md。
5. **总验收判据 1 预检** ✅：用户原话场景单探针（0.2.25）——0 次 .go 搜索 + 首屏有结论句 + 零写盘 completed；正式 G-A 门跑测仍由验收窗独立复跑。
6. **归跑测阶段的判据**（本窗不预支）：C 语料 L285 同错计数（判据 2/3 机器读数）、T-1 决策门 A/B 三指标、G 门正式跑测与砺判读。机制层判据全部单测锁定（可证伪 + 改前必红已实证）。
7. **R5-2 字面偏差**：ERROR 行只带 class（记录时点可诚实产出），strategy/suggestion 由 R5-1 注入块同轮送达——信息时序约束，见 R5-2 战报自检。

