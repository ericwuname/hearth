# hearth-slim 手术包二 · 八卡战报 v1.0（traecode → 顶层 · 验收窗）

施工依据：《hearth-slim 手术包二·长程任务书 v1.0（顶层签发·traecode 执行）》（内容 v1.1，
八卡）。本战报对应 traecode 全权执行结果，供执行窗对照判据验收、顶层复核。

---

## 一、交付清单（八卡 · 八 commit · 施工顺序 S7→S8→S12→S9→S10→S11→S13→S14）

| 卡 | 内容 | commit | 一句话证据 |
|---|---|---|---|
| S7 | provider 瞬时故障分级长退避重试（30s→1m→2m→5m…/30 分钟窗口；窗口耗尽=暂停） | `6c3d286` | 分类表测试 + 重试投影测试 + 窗口耗尽 paused 测试 |
| S8 | run 级断点续跑（每步落盘 run 执行位 / `hearth resume` / 统一暂停语义） | `0830173` | run_state roundtrip + 每步 checkpoint 触发 + paused 终态断言 |
| S12 | 交付前质量自检（类型化检查 → 回喂修复 ≤2 轮 → 诚实交付） | `b15d4a7` | 语法错被捕获并修复 + 轮次用尽诚实交付测试 |
| S9 | provider 降级链（**仅** fatal/param 切通道；transient 交回 S7） | `df6abec` | 切/不切两向测试 + `[fallback]` 投影 + keys 0600 |
| S10 | 流式输出 + 三通道分层渲染（思考/执行/结果） | `3963be5` | 流式投影 + 降级非流式 + reasoning 不入历史测试 |
| S11 | 中断保留上下文（Ctrl-C 立即停、上下文保留、双击退出 REPL） | `7f3d27d` | 步边界打断 + in-flight 打断（agent 交还、历史保留）+ 双击规则测试 |
| S13 | 内置联网搜索 `web_search` + **全部工具**描述五要素审计 | `fd50e77` | 解析器 mock HTML 测试（DDG/必应/去重截断/畸形容错）+ R5-9 全量（10 工具）审计 |
| S14 | 任务总结报告 TL;DR（六段模板 / 降级不阻断 / `/summary`） | `d30c48a` | 生成式总结 + 模型不可用降级 + 打断轮跳过额外调用 + 报告头部断言 |
| S14·修 | S14 收尾回归修复（service F1 spy 断言 + 总结终态标签） | `c3e5ba7` | service integration 9/9 复绿；标签恒非空 |

一句话总览：**韧性（S7/S8/S9/S11）→ 交付质量（S12/S14）→ 可用性手感（S10/S13）**，
八卡全部落地并各自独立 commit（外加 S14 的收尾回归修复一枚）。

---

## 二、红线自查（顶层四红线）

1. **沙箱 crate 语义零触碰**：`crates/sandbox` 本批零改动（`git log -- crates/sandbox` 无本批提交）。
2. **`a_arm_act_tally` 9 处原位**：`loop.rs` 现 9 处引用（1 定义/1 调用点/7 测试），数量与位置未变。
3. **key 永不入 git**：八卡提交仅含 `.rs` 源码；`provider_keys`（0600）只落在 `~/.config/hearth/config.toml`，
   仓库内零凭据文件（`git show --stat` 逐一核对）。
4. **S5 已拆机制禁止复活**：本批零新增相位机/强制 decompose/强制 write 类机关；
   消息循环仍是唯一主路径（S7/S11 的打断与重试都挂在既有循环上）。

---

## 三、门禁（每卡都过）

- 每卡：`cargo fmt --all` 干净 → `cargo clippy --all-targets`（agent-core / codex-cli /
  tools-builtin / planner / agent-types）**零警告** → 新测试先红后绿 → 独立 commit。
- 收官整包（`cargo test --workspace --no-fail-fast`）：
  - **绿**：agent-core **138/138**、codex-cli **33/33**、service integration **9/9**、
    planner / agent-types / llm-gateway / tool-runtime / project-sync / bridge 等全绿。
    tools-builtin 新增 7/7 + R5-9 全量（10 工具）审计通过。
  - **红（4 个 target，全部环境/既有，非本批代码）**：tools-builtin 11 项（POSIX bash +
    Unix 绝对路径语义）、sandbox `test_noop_sandbox_echo`（需 POSIX `echo`）、
    project-xray `real_workspace_wiring_all_green`（读 `docs/xray/wiring-v13.toml`，
    该文件属工作区既有删除项）。详见第四节问题卡。
- 回归如实申报：①R6-9 A 臂"恰 2 次模型调用"断言因 S14 总结调用 **2 → 3**
  （任务书显式口径"总结调用计入本 run tokens"），同 commit 更新并新增总结块随
  report 交还的断言；②`service` F1 多轮上下文断言因"最后一次 ChatRequest 被总结
  调用覆盖"而红 → `c3e5ba7` 修复（spy 改累计 + 断言排除总结调用），非放宽断言。

---

## 四、环境问题卡（申报，不静默绕过）

**问题卡 1 · .133 Linux 不可达（贯穿八卡）**
- 现象：任务书声明"测试与构建一律在 .133 Linux 跑"，但本机 ssh 免密
  （`Permission denied (publickey,password)`）与 paramiko 空密码（`AuthenticationException`）均失败，
  .131 同样不可达——执行侧**无凭据**进入两台 VM。
- 处置：按顶层已裁定的**本机 MSVC（Windows）**标准路径构建/测试，每卡 commit 内如实标注；
  不静默宣称"已在 .133 验证"。
- 影响面：编译/单测门禁在本机成立；**真机（.133）行为留待执行窗复验**（见第五节）。

**问题卡 2 · 全 workspace 在本机 Windows 的既有红（4 target / 13 项，与本批零交集）**
- tools-builtin 11 项 = 10 项稳定环境红 + 1 项负载抖动：
  - 稳定红：`bash::tests::{test_bash_echo, test_bash_tail_normal_output,
    test_bash_node_check_real_exit_code}`（需 POSIX shell）与 `read::test_read_path_traversal_denied /
    edit::test_{edit_path_traversal_denied, edit_absolute_path_outside_workspace_denied} /
    grep::test_{grep_traversal_path_rejected, absolute_path_within_root_allowed} /
    p3_tests::test_rc25_{default_roots, explicit_roots_replace}`（Unix 绝对路径语义——
    `/etc/passwd`、`/etc/shadow` 在 Windows 非绝对路径）。
  - 抖动：`bash::tests::test_bash_timeout`——单独跑通过、整包并行跑时红（超时时序对负载敏感），
    非确定性，非本批引入。
- sandbox `tests::test_noop_sandbox_echo`：`program not found`（需 POSIX `echo`）。
- project-xray `real_workspace_wiring_all_green`：读 `docs/xray/wiring-v13.toml` 失败——
  该文件属**工作区既有删除项**（工作区有 1461 项 docs 删除未提交，非本批引入），
  与八卡代码零关系。
- 处置：申报，不在本批"修绿"（防把环境/既有问题伪装成代码问题）；预期 .133 Linux 上为绿。

---

## 五、执行窗遗留（申报不代办 —— 需真机/人工终评）

1. **S12 判别实验**：魂斗罗"原版 vs 质量版"对照，用户终评（任务书明列的用户判据）。
2. **S10 真机目测**：打字机流式观感 + 三通道分层（思考/执行/结果）可读性。
3. **韧性三场景真机**：`kill -9` → `hearth resume`；断网 2 分钟后恢复续跑；401（key 失效）→ paused + 换 key 续跑。
4. **S11 真机演练**：任务中 Ctrl-C → 立即停 → 追问"刚才做到哪" → 模型基于完整上下文回答；
   连按两次 Ctrl-C（3s 内）退出 REPL。
5. **S13 真机取证**：任务中模型自主调用 `web_search` 并引用结果（需网络可达）。
6. **S14 目测验收**：任务收尾总结块 + `/summary`（用户判据"内容多也可以不看过程"）。
7. **吴涛 3 真实任务抽查**（总验收）。

---

## 六、偏离与取舍（显式申报，非静默）

1. **egress 双语义并存**（S13）：`web_search` 按任务书"出网走 S2 默认放开"（空白名单=放开，
   显式白名单=收紧）；`web_fetch` 仍是历史 fail-closed（空白名单=全拒）。两套语义并存是既有事实，
   本卡未擅自统一（防范围扩散）——如需统一口径，建议单独立卡。
2. **S11 工具执行边界**：Ctrl-C 在工具**执行前**收手（不 drop in-flight 工具——避免半成品写盘）；
   正在跑的长命令在下一步边界停住。模型调用中的打断是即时的（notify）。
3. **S14 打断轮不发起总结调用**：Ctrl-C = 立即停，再打一次 provider 违背用户意图 →
   该路径走机械降级块（产物/自检/剩余建议仍为实数据），块内显式说明原因。
4. **S14 SSE 路径**：总结随 `report` 交还并由 CLI 投影/落报告；service SSE 的 Done 事件在
   总结生成之前发出，故 **SSE 消费端本轮不额外渲染总结块**（报告文件内仍有）。CLI 直跑
   （`hearth chat` / `hearth repl`）是任务书验收面，已覆盖。
5. **S13 `web_search` 不引新依赖**：解析器/实体解码/percent 解码自实现（零依赖增量）；
   网络可达性不打单测（解析器喂 mock HTML），真机取证见第五节。

---

## 七、对顶层的请求

1. 八卡是否**放行进入执行窗验收**（第五节 7 项）。
2. 问题卡 1（.133 不可达）是否维持"本机 MSVC + 执行窗真机复验"的处置，或另开凭据通道。
3. 偏离第 1 条（egress 双语义）是否单独立卡统一。
4. **待顶层处置的既存事项（本批未碰）**：工作区有 **1461 项 docs 删除**未提交（含
   `docs/xray/wiring-v13.toml`，已导致 project-xray 一项测试红）与若干未跟踪产物目录
   （`.playwright-cli/`、`bench/exam/*.png`、`crates/agent-core/.hearth-diag/`）——
   本批只提交八卡相关源码，未擅自恢复/删除这些既有变更，请顶层裁定处置方式。
