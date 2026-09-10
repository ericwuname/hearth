# hearth-slim 第一批手术·施工对接单 v1.0（执行窗→traecode）

- **日期**：2026-09-09 21:10　**效力**：顶层签发件 c343031 + 计划书 cbe4dea 的执行窗转达件——**trae 收到本单即开工**
- **施工依据（必读三件）**：
  1. `docs/p0-usability/r9-reports/hearth-slim第一批手术·长程任务计划书 v1.0（执行窗拟→顶层签发→traecode）.md`（cbe4dea）——六卡全文+锚点+过关判据
  2. `docs/p0-usability/r9-reports/hearth-slim 第一批手术计划书·顶层签发件 v1.0（顶层）.md`（c343031）——顶层四增补（见 §三）
  3. 本单 §四 验收协议与命令模板

---

## 一、施工环境与基线

| 项 | 值 |
|---|---|
| 仓库 | `C:/Users/87465/Desktop/codex-rust-v1.0-final`，分支 `p0-usability-01` |
| 施工基线 | tag `v0.2.26-final`（HEAD a533e81 后缀，loop.rs **8,114 行**） |
| 开工前置 | `git archive --format=tar.gz -o ~/Desktop/hearth-slim-backup-$(date +%Y%m%d).tar.gz HEAD`（每卡落刀前重做） |
| 门禁命令 | `cargo fmt --all -- --check`（rc=0）+ `cargo clippy -p agent-core -p planner -p agent-types --lib 2>&1 \| grep -c "^error"`（=0） |
| 测试命令 | `cargo test -p agent-core --lib --no-fail-fast`（注意：多包默认 fail-fast，必须带 --no-fail-fast） |
| 环境警示 | 本机 GNU 工具链缺 dlltool（冷启动编译挂）——用增量缓存或切 MSVC（`cargo +stable-x86_64-pc-windows-msvc`） |

## 二、施工顺序（六卡，逐卡独立 commit）

```
S1 工具超时（小）→ S2 出网默认放开（小）→ S3 prompt 瘦身（中）→ S4+S5 give_up+相位机同刀（大）→ S6 compaction 调参（小）
```

各卡病灶锚点、动作明细、过关判据——**以计划书 cbe4dea 为准**，本单不重复。卡序不可调换（S1/S2 独立先行，S4 大刀必须在 S3 后——prompt 瘦身先行给大刀降低上下文噪声）。

## 三、顶层四增补（c343031，施工中硬约束）

1. **宪法注入已正式移出"旧领先资产"清单**（实测膨胀元凶，语义降级投影层而非删除）——**禁止从旧台账翻案**，S3 卡落刀时不再需要"保护宪法注入"的旧义务；
2. **`a_arm_act_tally`（9 处，loop.rs）在 S4/S5 中有耦合**：如手术必须触碰 → **申报**，不许顺手删除（R8 战果资产）；
3. **S4 大刀前置令**：落刀前归档（§一命令）+ 独立 commit（不与其他卡混）+ **卡住超 1 个工作日 = 申报拆分方案**（不硬凿）；
4. **销账判定权归顶层**（按 c070411 口径独立裁定）——trae 与执行窗均**不自宣**达标；执行窗出复测数据包，顶层终裁。

## 四、验收协议（每卡交付物 → 执行窗验收）

| 卡 | trae 交付物 | 执行窗验收动作 |
|---|---|---|
| S1 | 超时实现 + 先红后绿单测证据 + commit | 抽验：构造 sleep 长任务实测回收；grep 超时机制在位 |
| S2 | 白名单语义反转 diff + 审计投影 + curl 单测 | 抽验：真机 curl example.com 通 + `system_chars` 复测 |
| S3 | 注入段删改 diff + `system_chars ≤2,000` 真机截图/log | 实测 introspect system_chars；宪法出注入层锚点核对 |
| S4+S5 | 大刀 commit（独立）+ 新闭环测试集（先红后绿证据）+ 删改清单 | 门禁复跑 + `grep LoopPhase\|give_up` 残留核对 + 最小会话真机 + tally 9 处原位核对 |
| S6 | 阈值参数 diff + 长任务 tokens 峰值数据 | corpus 单条 max < 80K 复测 |
| 总验收 | 复测数据包（run-regression 同款协议，双语料+L1-01） | 执行窗判分 → **呈顶层销账终裁** |

## 五、问题卡制度（卡住即申报）

- 任何卡超出预估时长 / 遇到与本计划书冲突的现状 / 需要触碰红线边缘 → **停手，出问题卡**（现象+锚点+建议方案），等执行窗/顶层回应；
- 禁止静默绕过、禁止顺手改卡外内容、禁止为过测试改测试断言（测试改判据须申报）；
- 已知问题卡参考：C-fix-status 挂死（S1 的直接动因）、GNU dlltool 缺失（§一环境警示）。

## 六、红线（违反即停工）

1. 沙箱 crate 零触碰（4.5/5 资产）；
2. `a_arm_act_tally` 9 处原位（触碰须申报）；
3. 每卡独立 commit + 备份前置；
4. 判据不许改——改判据=申报顶层（防墙头草）。
