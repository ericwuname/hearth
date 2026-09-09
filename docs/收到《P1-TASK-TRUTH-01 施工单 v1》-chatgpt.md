收到《P1-TASK-TRUTH-01 施工单 v1》。

整体方向批准，但**暂不直接开工**。施工单需要先做以下 5 项开工前修正；修正完成后无需再次逐节点汇报，直接按长程总包模式执行。

### 1. Node 03：先完成 Acceptance Criteria Producer Audit

在真正实现 verifier 前，先源码核实：

```text
acceptance_criteria
    ↓
谁生产？
哪些入口可以写？
provenance 是否明确？
resume 是否保留？
goal revision 是否会覆盖？
```

如果已有权威生产入口，必须复用。

如果当前实际上只有字段、没有可靠 producer：

**不要自行新建第二套 criteria 系统；触发 STOP-8，提交偏差说明。**

另外：

`cmd:` criteria 必须明确为**验证命令，不是工作命令**。

不能因为 acceptance verification 而产生新的执行能力。

至少禁止 verifier 通过 criteria 引入：

```text
rm/mv/cp
redirect write
tee
chmod/chown
mkdir/touch
设备写入
网络访问
pipe-to-shell
其他产生副作用的命令
```

验证命令必须继续经过现有 bash/tool-runtime/approval/sandbox 边界，禁止绕过任何既有安全层。

若当前无法建立安全、可审计的 cmd verifier：

**STOP-9。**

---

### 2. Node 05：禁止“免费预算豁免”

当前：

```text
budget-low
+
acceptance verification 未完成
```

不能通过简单“标记位豁免”无限继续，否则虽然没有修改 budget 数字，实际上已经扩大有效预算。

改成：

# Verification Reserve

必须从现有总预算语义中划分有限验证额度，而不是新增无限预算。

要求至少同时满足：

```text
verification attempts <= 现有 verify_replan 上限
remaining deadline 继续有效
不得突破原始任务预算总量
不得重新打开普通 execution budget
```

如果设计最终必须增加总 budget cap 才能实现：

**STOP。**

Node 05 必须先通过源码级影响面分析，再实施。

---

### 3. Node 04：真实冲突样本不要人为凑数

要求：

> 尝试收集 ≥3 个自然产生的真实 REFLECT_FACT_CONFLICT。

如果自然样本不足：

* 不得人为制造生产级错误；
* 可以使用 replay/mock fixture 补足“观测链是否成立”的证明；
* 最终报告分别写：

  * natural conflicts
  * replay/synthetic conflicts

不能因为“必须三个”而污染实验。

---

### 4. Node 07：把“0 conflict”改成“0 unresolved conflict”

本阶段的目标之一正是发现 Fact ↔ Reflect 冲突。

因此：

```text
出现 conflict
→
经过 verification
→
成功消解
→
最终 completed
```

应该可以判为成功。

真正不能接受的是：

```text
Fact / Verification 已明确
+
Reflect 冲突
+
没有消解
+
错误终态
```

所以 Node 07 的硬条件改为：

```text
no unresolved REFLECT_FACT_CONFLICT
```

而不是：

```text
zero REFLECT_FACT_CONFLICT
```

同理，“零假声明”建议保留为观察指标；真正验收依据必须是 L1/L2/L3/L4 结构化证据，而不是要求整个真实 LLM run 绝对没有任何文本错误。

---

### 5. O-4 命名与边界

将：

`O-4 Semantic Completion`

改名为：

# `O-4 Deterministic Acceptance Verification`

因为本 Node 实现的是：

```text
exit code
file existence
file content
nonempty
acceptance criteria
```

属于确定性验证，不是 LLM semantic understanding。

真正的：

```text
semantic interpretation
Fact ↔ Reflect authority
```

继续归 DEV-2。

禁止因为 O-4 完成而声称“Semantic Completion 已解决”。

---

### 修正后的核心事实链必须保持：

```text
User Goal
↓
Task Type
↓
TaskGraph
↓
Tool Evidence
↓
Artifact Evidence
↓
Acceptance Verification
↓
Reflect Interpretation
↓
Completion Decision
↓
Terminal
↓
Projection
```

其中：

> **Reflect 不得单独成为 Completion 的最高事实权威。**

本轮可以继续 Observe-only 研究 Fact ↔ Reflect 冲突，但任何“Fact override Reflect”或“Reflect override Fact”的新控制流都必须触发 STOP。

---

以上 5 项修正完成后：

**直接进入 Node 00 → Node 11 长程连续执行。**

Node 之间不得等待用户确认。

普通 test failure / clippy / fmt / fixture failure / provider 单次失败：

```text
diagnose
→ fix
→ retest
→ continue
```

只有 STOP-1~9 触发时才暂停并向顶层报告。

最终一次性提交：

`P1-TASK-TRUTH-01 Final Report`

---

## 守门员复核与开工前补充（2026-08-30 08:05 · 与正文同效力，修正后开工）

> 复核记录：`acceptance_criteria: Vec<String>` 与三态函数 `acceptance_verification_status()` 实锤（loop.rs:1297/:1435-1436）："passed 无生产者"前提与 :6926 测试一致——Node 02 表格成立；ChatGPT 五项修正全部同意。**抓到一处事实漂移**：施工单 Node 03/05 称 verify_replan 额度"≤3 次"，源码实际为 **`verify_replan_count < 1`（loop.rs:2376，只给一次）**——Reserve 定额与 Node 03 回喂额度必须对齐源码真实值，禁按施工单笔误实施。以下 5 条补充。

### 增 1（安全，最重要）：`file:` 核验的路径越权与资源风险——施工单没写，必须补

Node 03 的 `file:` 条目"走确定性读取"——若实现为进程内 `std::fs` 直读，**不经过沙箱**（landlock/seccomp 只约束 bash 子进程），criteria 即可指向 `/etc/shadow` 等任意路径让 verifier 代读。钉死两条：

```text
① file: 目标必须落在 verify_written_files 同一白名单（workspace/writable_paths）内
   ——越界条目判 invalid（结构化记录），不读；
② 单文件读取上限（沿用 bash 截断的 64KB 语义）——防大文件读爆内存。
```

另注意 criteria 的**生产者若含模型**（planner/LLM 生成），上述越权就不仅是理论风险——Producer Audit（修正 1）把"criteria 谁写的"一并回答。

### 增 2（安全）：cmd: 的写盘快照检测比禁名单更硬

修正 1 的副作用禁名单是**字符串匹配**（RC24 `> /dev/` 一刀切的老路——base64/变量展开/管道都能绕）。保留禁名单作第一道粗滤 + BashRisk HardRedline 第二道之外，加一道**确定性兜底**：

```text
验证命令执行前对 workspace 做写盘快照（文件清单 + mtime）
执行后比对：出现新增/变更文件 = verifier 越权写
→ 该条 criteria failed + 结构化记录（"verification wrote files"）
```

"验证命令不得产生工作产物"从禁名单约定升级为可检测的不变量——禁名单漏花样也漏不过快照。

### 增 3：Producer Audit 与 revision-mismatch 既有裁决衔接

修正 1 的审计问题之一"goal revision 是否会覆盖 criteria"——R2-B 时代已有裁决：revision mismatch 时 original_goal+graph 照常采用、constraints/criteria **标注 stale + warning，不阻断不静默**。Audit 直接核对现状是否已实现该语义，缺则补（属兼容扩展范围），勿另起炉灶。

### 增 4：Reserve 消耗必须可观测

Node 05 的 Verification Reserve 若落地：**reserve 消耗在 telemetry 单独记账**（每笔 reserve replan 带标记）——"有效预算扩大"如果发生，必须从数据上可见而非隐式。额度上限对齐源码真实 verify_replan 值（见复核漂移）。

### 增 5：延续纪律确认

Node 00/09/10 已含归档、df 保险丝、gate log 路径登记、系统 PATH 口径、分窗登记、bump 前置——齐了，无需再补。开工。
