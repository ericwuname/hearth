# P4 Node 14 测试派工单 v1.0（执行窗 → 测试窗口）

**发出**：执行窗（v0.2.21 施工者）｜**日期**：2026-09-01 17:40｜**被测版本**：**v0.2.21**
**性质**：只派活与定口径，**不含疗效判读**。疗效结论与 α/β/γ 议题归顶层 / 外部 AI / 砺。

---

## 0. 一句话任务

在 **v0.2.21** 上**独立复测 RC52**：检验「跨 run 产物事实回填」修复是否真的压低了 fresh 失败率。
跑 **A（干净控制）×5 + B（污染重放）×5**；**C（resume）仍 BLOCKED**，待砺复核 R-1 仪器后解封再补。

---

## 1. 环境核对（开工三查，缺一不跑）

```bash
bash -lc 'hearth --version'          # 期望 hearth 0.2.21 (unknown)
md5sum /usr/local/bin/hearth         # 期望 5e5ac1a327d806f27af77b2e8b89f14e
ls -l ~/.config/hearth/sessions/*.jsonl | wc -l   # 会话目录可写、有历史会话
```

| 项 | 值 |
|---|---|
| 机器 | **.131（唯一环境）**，不要换机器 |
| 版本 | v0.2.21（commit `1366710` / tag `v0.2.21`） |
| 二进制 md5 | `5e5ac1a327d806f27af77b2e8b89f14e` |
| provider | `agnes` / `agnes-2.5-flash`（config.toml 默认） |
| **真机跑批必设** | `HEARTH_ALLOW_NO_CGROUP=1`（脚本 `env()` 已自带，**不要删**） |

> ⚠️ 这条变量分作用域，别套错：**跑 `cargo test`（门禁）不要设**（与 sandbox `test_rt4_cgroup_fail_closed` 互斥）；**真机跑批必须设**——不设则 bash 被 RT4 fail-closed 拒绝，轨迹从 9 步变 5 步，与 v0.2.20 基线不可比。

## 2. 开工前五项检查

```bash
# ① provider 探活（VPN 是否生效；401 = 端点存活可用，超时 = 不可达）
curl -s -o /dev/null -w 'agnes=%{http_code}\n' --max-time 15 https://api.agnes-ai.cn/v1/models
curl -s --max-time 12 http://myip.ipip.net        # 出口 IP（当前经 VPN：45.88.202.26 挪威）

# ② 版本与二进制（见 §1）
# ③ 资产齐备
ls ~/fa/simuser/          # driver.py / analyzer.py / run_rc52_matrix.py / metrics.yaml
ls ~/fa/campaign/golden/  # replay41.txt + 手工测试v0.2.18.txt

# ④ 目录隔离（见 §3，禁止复用旧数据目录）
# ⑤ 会话目录可写
test -w ~/.config/hearth/sessions && echo writable
```

**VPN 注意**：出口已改走挪威（原国际出口被拦）。若跑批中途 VPN 掉线，会出现
`error sending request` → 该样本判 **PROVIDER_UNREACHABLE**，**不得计入失败率分母**，需重跑。

## 3. 目录隔离（强制）

```bash
mkdir -p ~/fa/p4-rerun2 && cd ~/fa/p4-rerun2
cp ~/fa/simuser/driver.py ~/fa/simuser/run_rc52_matrix.py ~/fa/n14/n14_judge.py .
sed -i 's#^OUT = .*#OUT = "/home/wutao/fa/p4-rerun2"#' run_rc52_matrix.py
grep -n '^OUT = ' run_rc52_matrix.py            # 确认已指向新目录
```

- **禁止**写 `~/fa/p4`（执行窗 v0.2.20 数据）与 `~/fa/p4-rerun`（测试窗旧数据）——混表风险。
- **C 条件删行**（BLOCKED 期间）——⚠️ **v1.0 初版给的 sed 有 bug，已更正，请用下面这条**：
  ```bash
  # 推荐：python 改写 + 锚点断言 + 语法校验
  python3 - <<'PY'
  p = 'run_rc52_matrix.py'
  s = open(p).read()
  old = '+ [("C", i) for i in range(1, 4)])'   # 注意行末是 ]) 而不是 ]
  assert old in s, '锚点未找到，勿盲改'
  open(p, 'w').write(s.replace(old, ')'))
  PY
  python3 -c "import ast; ast.parse(open('run_rc52_matrix.py').read()); print('SYNTAX OK')"

  # 等价 sed（已实测通过）
  sed -i 's#+ \[("C", i) for i in range(1, 4)\])#)#' run_rc52_matrix.py
  ```
  目标结构：`plan = ([("A", i) for i in range(1, 6)] + [("B", i) for i in range(1, 6)])`。
  **历史教训（本窗自陈）**：初版写的是 `sed -i '/("C", i) for i in range(1, 4)/d'`，会把行尾 `])` 一并删掉 → `SyntaxError`，测试窗已现场修复。**今后任何改跑批脚本的动作，改完必须 `ast.parse` 校验再启动。**

## 4. 预注册判据（先读，防止事后叙事）

| 项 | 内容 |
|---|---|
| **主判据** | v0.2.21 fresh（A 条件、REPL「继续」链路）RC52 复现率**显著低于 v0.2.20 的 40%** |
| **反向信号** | 若仍 ~40% 或更高 → **判修复无效，回炉**，不许追加补丁掩盖 |
| **分母锁定** | REPL 内「继续你的提议吧」（无显式 goal 恢复）才是病灶链路；`hearth resume`（显式 revision 1）结构性绕过病灶，**不能替代 A** |
| **方差提醒** | RC52 是概率性吸引子（同二进制同输入曾出现 completed/failed 两种终态），40% 不可当精确门槛，看**方向与非重叠区间** |
| **样本量** | A×5 + B×5（与 Node 03 口径一致）；若结果落在 30–50% 灰区，**扩到 10 跑/条件**再判 |
| **⚠️ pilot 分母口径（v1.1 新增）** | §5.1 的 **pilot A1 是通路检查、不计入样本**——但它若出现 `failed`，**必须在报告里单独披露**（不得隐去）。已实测：pilot A1 = failed、矩阵 A1 = completed（同码同输入异果，非确定性的直接证据）。**主判据以矩阵 10 跑为准，pilot 只作非确定性披露项。** |

## 5. 执行

```bash
# 5.1 先单跑 A1 验通路（约 30s；看到 JSON 行含 task_terminal/continue_terminal 即通路正常）
cd ~/fa/p4-rerun2 && python3 - <<'EOF'
import sys; sys.path.insert(0,'.')
import run_rc52_matrix as m
print(m.run_a(1))
EOF

# 5.2 全矩阵后台跑（A×5 + B×5；B 需逐字重放 13 条前缀，整体约 25–40 分钟）
cd ~/fa/p4-rerun2 && cp ~/.config/hearth/diagnostics.log diagnostics.before.log
setsid nohup python3 run_rc52_matrix.py > matrix.log 2>&1 < /dev/null &

# 5.3 监控（每跑输出一行 JSON；末尾 MATRIX_DONE）
tail -f matrix.log
```

判定与自检：

```bash
# 5.4 确定性判定（口径已与砺 Step3 人工结论对齐：旧数据 9/10 复现、B4 判 DRIVER_INDUCED）
cd ~/fa/p4-rerun2 && python3 n14_judge.py ~/fa/p4-rerun2/results.json --logs .

# 5.5 回填发生频次（客观计数，诊断日志）
grep -c RC52_ARTIFACT_HYDRATION ~/.config/hearth/diagnostics.log      # 跑后
grep -c RC52_ARTIFACT_HYDRATION diagnostics.before.log                # 跑前（差值即本轮回填次数）
```

**判定桶定义**（`n14_judge.py`，确定性字符串/计数，模型"感觉"不作证据）：

| 桶 | 含义 | 是否入分母 |
|---|---|---|
| `RC52_REPRO` | CONT 后终态 failed（病灶复现） | ✅ 入 |
| `CLEAN_PASS` | CONT 后终态 completed | ✅ 入 |
| `TASK_FAILED` | 首轮就没完成（非 RC52 形态） | ✅ 入 |
| `DRIVER_INDUCED` | CONT 段无终态标记（driver `wait_turn` 只认 completed/failed） | ❌ 剔除 |
| `PROVIDER_UNREACHABLE` | 日志含 `send request failed` / `error sending request` | ❌ 剔除 |
| `RUNNER_ERROR` | 跑批脚本自身异常 | ❌ 剔除 |

## 6. C 条件（resume）：仍 BLOCKED

- R-1 仪器已修并通过干跑自证（**8/8 PASS**：空目录→空串使断言可触发 / 新增 uuid 可取回 / 不误取旧会话 / 追加可检出 / 取 mtime 最新者 / 真实 sessions 目录 264 个全为 36 位 uuid）。
- 复跑脚本：`python3 /tmp/r1_instrument_dryrun.py`（退出码 0 = 全过）。
- **执行窗不自解封**：请砺复核通过后，再恢复 C 行跑 C×3。

## 7. 必采字段（每跑）

既有（driver/results.json 自动落）：`cond` / `task_terminal` / `continue_terminal` / `session_ids` / 全量终端 log / planner dump。
**本轮新增三项**：

1. **`RC52_ARTIFACT_HYDRATION` 计数**（§5.5）——回填是否真的发生、发生几次。
2. **截断标记出现位置**：日志中出现 `[... N chars truncated]` 处（宪法注入 / goal_drift / failed_nodes / CLI 诊断错误）——信息销毁可见化的产物，非新缺陷，但需记录。
3. **provider 侧状态**：不可达/重试次数（用于把环境故障从失败率里摘出去）。

## 8. 红线

1. **不得修改 benchmark**：`TASK` / `CONT` / `PREFIX`（13 条污染前缀）与判定脚本一律不得改——改了数据即失效。
2. **不得改 `crates/` 源码**（改了就不是 v0.2.21）。
3. **不得复用/覆盖旧数据目录**（§3）。
4. **不自判疗效**：测试窗只出**数据与分类**，结论交砺与顶层（判读分离）。
5. **不得把 provider 不可达或仪器缺陷计入失败率**（§5 表）。

## 9. 交付物与落点

```
~/fa/p4-rerun2/
├── results.json              # 每跑结构化结果
├── p4_A{1..5}.log            # A 条件全量终端日志
├── p4_B{1..5}.log            # B 条件全量终端日志
├── matrix.log                # 跑批过程输出
├── diagnostics.before.log    # 跑前诊断快照（算 hydration 差值）
└── 判定输出（n14_judge.py stdout，请原样保存）
```

报告要求：**数据表 + 判定器输出 + 与预注册判据的对照 + 任何偏离本派工单的说明**（改了什么、为什么）。
分发：交**砺·评审**判读 → 顶层 / 外部 AI 做 α/β/γ 终审。

---

## 附录：命令速查

```bash
hearth --version
md5sum /usr/local/bin/hearth
curl -s -o /dev/null -w '%{http_code}\n' --max-time 15 https://api.agnes-ai.cn/v1/models
python3 /tmp/r1_instrument_dryrun.py                       # R-1 仪器自检（退出码 0 = 全过）
python3 ~/fa/n14/n14_judge.py <results.json> --logs <dir>   # 确定性判定
```

**门禁环境（若需复跑单测，注意与跑批不同）**：
```bash
cd ~/codex && source ~/.cargo/env && unset HEARTH_URL \
  && export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth && unset HEARTH_ALLOW_NO_CGROUP \
  && cargo test --workspace        # 基线 460 passed / 0 failed / 61 targets
```
