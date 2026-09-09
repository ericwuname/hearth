# 红队审计发现报告：v22.0 双引擎

> 日期：2026-08-02 | 审计依据：`docs/test-plan-audit-v22-final.md`
> 被测：window-framework v1.0.3（177 checks）/ codex-rust v22.0（214 passed）/ codex-cli v22.0
> 基线（`git log` 实测）：本地 HEAD **1255498**，tag **v22.0 → 8e39775**。本文所有 file:line 以此为准。
> 方法：守门员三层（存在性→行为→表达力）。证据分两类：
> - **【动态】** = 本窗口写了探针脚本真正执行复现（本地 Python / VM paramiko）。
> - **【静态】** = 直接读源码 + 控制流推演坐实（file:line），动态探针进行中或需特定环境。
>
> **v2 修订（2026-08-02 01:05）**：codex-rust 全部动态项已在 VM 复跑坐实。
> 首轮 VM 探针（`v22_audit_vm.py`）自身有 3 个 bug（`env` 接子 shell 语法错致服务从未启动、
> 429 统计管道接错、`echo EXIT=$?` 取的是 `tail` 的退出码），其结论**全部作废**；
> 现结论以 `v22_audit_vm4.py` 为准。**审计工具自身也必须被审计** —— 首轮 CT1 的
> 「exit=0」是假阳性（`codex-xray` 当时根本没构建）。

---

## TL;DR

- **测试有效（满足验收标准）**：发现 **14 个 🔴 可利用缺陷** + 多 🟡 契约不符 + 3 处 🔵 文档失真。红队打中了。
- 最高危：**审批门默认无鉴权 + CLI 字段错配 422（审批完全失效）**、**wiring 子串匹配可被注释绕过**、**stale-working 工作流假完成**、**`bash` 工具零沙箱**、**prompt 注入破坏 TOML**、**重启后会话全 404（数据已落盘但读路径未接线）**、**civ 告警三层静默丢弃**。
- 文档失真：version-iteration-manual 自相矛盾（214/15-15 vs 205/14-14）；acceptance 报告把未真正修复的旧债标「done」；README 回归命令漏跑 v09/v10。
- **v2 新增 2 项**（首轮遗漏，源码+动态双证）：**N10** `codex-cli sessions` 子命令打 GET，服务端只注册 POST → 405 恒失败；**N11** `MEMORY_DIR` 不可写时服务 `.expect()` 直接 panic 退出。
- **首轮 CT1 结论是假阳性并已推翻重测**：真值仍为「可绕过」，但首轮的证据链无效。

### 动态复现一览（VM 192.168.220.131，`v22_audit_vm4.py`）

| 场景 | 实测 | 判定 |
|---|---|---|
| RT1 就绪探针 | 运行中 `chmod 000` 掉 MEMORY_DIR，`/readyz` 仍 **200**，日志 0 条 warn | 🔴 探针不会红 |
| RT4/N2 审批鉴权 | 零凭据建会话 **201**，审批请求直达 handler（404 SESSION_NOT_FOUND，**非 401**） | 🔴 鉴权全放行 |
| RT4/N1 CLI 契约 | 照 CLI 真实发包 → **422** ``missing field `decision` `` | 🔴 approve/deny 全废 |
| N10 CLI 契约 | `GET /api/v1/sessions` → **405**（Allow: POST） | 🔴 sessions 子命令全废 |
| RT9/N8 重启恢复 | 重启前 200 → 重启后 **404**；**但 jsonl 已在磁盘上** | 🔴 写了不读 |
| RT6/N9 限流 | 60 条慢连接 → **50×201 + 10×429**，限流器确在 50 触发，无 Retry-After | 🟡 全局额度可被单客户端吃满 |
| CT1 wiring | 基线 exit=0；注释掉调用后仍 **exit=0** | 🔴 断言可绕过 |
| RT8 回放 | `REPLAY DONE: 31/31 passed`（零 token） | ✅ 安全 |
| N11 启动韧性 | 坏 MEMORY_DIR → `panicked at main.rs:503` 进程退出 | 🔴 可恢复错误升级为崩溃 |

---

## A 组：window-framework（动态探针：` .workbuddy/window_audit_probe.py`，证据 `window_audit_evidence.jsonl`）

| # | 场景 | 严重度 | 结果 | 证据 |
|---|---|---|---|---|
| WT1 | 沙箱路径前缀旁路 | 🔴 | **已确认**【动态】 | `_allowed_write` 仅 `Path.resolve()`+`str.startswith`，无分隔符 → `outputs_evil/`、`outputsX/` 均被放行（framework.py:416-422） |
| WT2 | 产出验证绕过（空文件） | 🔴 | **已确认**【动态】 | `_has_outputs` 仅查目录非空，空文件即算产出（framework.py:457-462）；多窗口共享 `shared/outputs` 互证 |
| WT3 | provider 路由注入 | 🟡 | **已确认**【动态】 | 未知/大写 provider 静默回退 deepseek；缺 `[budget]` → KeyError 崩溃（framework.py:407-414）；路由不覆盖 compress/analyze |
| WT4 | budget 极端值 | 🟡 | 【静态】 | `max_steps=99999` + replay 模式完全不查预算 → 写 99999 行 + ~2000 次自动快照（DoS）；CLI 路径忽略 `budget.max_steps`（framework.py:552-563, 535） |
| WT5 | gate 时序 | 🟡 | 【静态】 | `--reject` 纯装饰：引擎只解析 `done:`，永不读 `blocked:`（framework.py:836, 995）→ reject 被绕过；重复 approve 非幂等 |
| WT6/N4 | working 残留假完成 | 🔴 | **已确认**【动态】 | 单窗口 stage 陈旧 `working` → 重置 pending 后 `continue` 跳过 `all_done=False`（framework.py:877-883）→ 自动 gate `echo VERIFY_PASS` 标记 stage done，**窗口从未运行** |
| WT7/WT9 | analyze 崩溃路径 | 🟡 | **已确认**【动态】 | `validate_deploy_yaml` 在缺 `id` 的 window 上 `w["id"]` 下标 → KeyError 未捕获（framework.py:1356，缺字段检查在 1376 之后）；fallback 内 HTTPError 逃逸 |
| WT8 | replay 全链路 | 🟡 | 【静态】 | `AGENT_MODE=replay` 全链路可跑；具体回归需在 VM 跑 |
| WT9 | reasoning_content 兼容 | 🟡 | 【静态】 | replay mock 含 `reasoning_content` → v1.0.1 已修 |
| WT10 | tokens 爆炸 | 🟡 | 【静态】 | 100KB read → `current_tokens` 按增量累计（v1.0.1 已修，缺回归保护） |
| WT11 | stage gate 生成 | 🟡 | 【静态】 | 仅 `auto:` 前缀生成脚本；裸路径 gate 不生成但 `_run_gate` 按脚本判 blocked；`gate_spec[5:]` 无校验 → 路径遍历（framework.py:2122-2128） |
| WT12 | 压缩空对话 | — | **安全**【动态】 | `compress()` 返回 1 graceful，rc=0（framework.py:1102-1104） |
| WT13 | 压缩质量退化 | 🟡 | **已确认**【动态】 | 连压 5 次对话行数序列 `[31,23,21,21,21]` 卡在 ~21，反复摘要放大（framework.py:1108-1153） |
| WT14 | 快照/回滚完整性 | 🔴 | 【静态】 | 快照只含 2 文件（不含 outputs）；回滚先 move 当前对话再判断 → 可净丢失；`--to` 路径遍历（framework.py:707-746） |
| WT15 | 导入导出往返 | 🟡 | 【静态】 | 导出截断（200 字符）有损；导入丢弃 `summary` role、重写时间戳、改形 tool_calls（framework.py:1673-1733, 1692） |
| WT16 | 运行时删窗口目录 | 🟡 | 【静态】 | `rm -rf windows/win-*/` → 崩溃可接受，但无 `.trash` 敏感信息残留检查 |
| WT17 | prompt 注入破坏 TOML | 🔴 | **已确认**【动态】 | `window create` 模板插值未转义换行/反斜杠 → 含 `\n` 的 prompt 使 `tomllib.loads` 失败，窗口永久不可读（framework.py:35-63, 222） |
| WT18 | 输出中继攻击 | 🟡 | 【静态】 | write_file 内容含 `[SYSTEM]...` 无净化，verify/gate 不捕获异常产出 |
| WT19 | analyze 重复 role | 🟡 | **已确认**【动态】 | `framework check` 与 `window create` 均不查重 role；仅 analyze/deploy 查（framework.py:79-134, 212-229, 1352-1354） |

---

## B 组：codex-rust（动态探针 `v22_audit_vm4.py`，证据 `~/codex_work/v22_vm_evidence4.jsonl`）

| # | 场景 | 严重度 | 结果 | 证据 |
|---|---|---|---|---|
| RT1 | /readyz 探活 | 🔴 | **已确认**【动态】 | 服务正常启动后把 MEMORY_DIR `chmod 000`，`/readyz` 仍 **200**，日志中 `session store unavailable` **0 次**。根因两层：① `JsonlMemoryStore::list_sessions` 用 `if let Ok(read_dir)` **吞掉全部 I/O 错**恒返 `Ok`（memory/src/lib.rs:222-233）；② 无 store 时 `list_persisted_sessions` 返回 `Ok(vec![])`（session.rs:128-133）。→ routes.rs:260-263 的 503 分支在生产路径**永不可达**，就绪探针形同虚设。单测 `test_epic_b_readyz_probe_semantics` 只用恒返 Err 的 `FailingStore` mock 覆盖该分支（integration_test.rs:1996-2024），故绿 |
| RT2 | civ 告警落盘 | 🔴 | **已确认**【静态】 | 三层静默：`drain_civ_alerts` 仅 `do_reflect` 一处调用；`civ_writer=None` → `civ_note` 无 else 丢弃（loop.rs:475-479）；append I/O 失败仅 warn（nervous-system/src/lib.rs:83-85, agent-core/src/loop.rs:1663-1670） |
| RT3 | sandbox landlock | 🔴 | 【静态】 | seccomp 拒绝清单仅 6  syscall（ptrace/reboot/init_module/delete_module/kexec×2），**不拦 socket/connect/execve/clone/unshare/mount**；landlock 失败仅 warn 继续；非 Linux → NoopSandbox 零隔离（sandbox/src/lib.rs:379-487, 294-375, 112-130） |
| RT4 | 审批门绕过 | 🔴 | **已确认**【动态】 | ① **零凭据建会话成功 201**，POST 审批直达业务处理器返回 `404 SESSION_NOT_FOUND`（**不是 401/403**），带伪造 `Bearer totally-wrong-key` 结果相同 → 默认 `API_KEY=None` 时 `require_api_key` 全放行（routes.rs:68-103, main.rs:526-545）；② 照 codex-cli 真实发包 `{approval_id, approved:bool}` → **422**，服务端原文 ``missing field `decision` ``（api/src/lib.rs:69-72 vs codex-cli/src/client.rs:135-138）；③ 提交未签发的 `approval_id="forged-not-issued"` 未被识别为伪造（dispatcher.rs:171-197） |
| RT5 | codex-cli resume | — | **安全**【静态】 | 不存在/done/cancelled/auth 均清晰报错，无 panic（codex-cli/src/main.rs:207-243, client.rs:284-321） |
| RT6 | 并发限流 | 🟡 | **已确认**【动态】 | 单客户端 60 条 `--limit-rate 5k` 慢速大 body 上传 → **50 条 201 + 10 条 429**，限流器确在第 50 条在途时触发。缺陷在设计：`P1_CONCURRENT_COUNT` 是**全局 static AtomicUsize**（routes.rs:271-284），无 per-IP/per-user 维度 → 单客户端即可吃满全局额度，被拒的第 51 条同样可能是他人请求。429 响应**不带 `Retry-After`**（回归门禁复测 10/10 个 429 全部无此头）。<br>⚠️ **动静分界**：「50×201+10×429」与「无 Retry-After」是**动态实测**；「健康端点不豁免限流」是**静态结论**（`concurrency_limit` 挂在整个 Router，main.rs:768；routes.rs:275-284 无 path 豁免分支）——**动态未能证实**：两轮探针（5k/2k 速率）中 `/healthz` 在慢连接期间均为 200，因 `--limit-rate` 只限客户端发送速率，300KB body 被 localhost 内核 socket buffer 一次吃下，handler 秒返，饱和窗口仅毫秒级。**勿据此认为健康端点已豁免。** |
| RT7 | 应力 8 场景 | 🔴(可选) | 【未跑·预算】 | 烧 token（~¥0.50），v22 runbook 已跑过 24/24 零 panic，本次省略 |
| RT8 | 回放 31 条 | — | **安全**【动态】 | `REPLAY_DIR=bench/replay/fixtures` 启 service → `bench/replay.py` 自报 **`REPLAY DONE: 31/31 passed`**，零 token 回放通道健康 |
| RT9 | 重启后会话残留 | 🔴 | **已确认**【动态】 | 建会话 → 重启前 `GET`=**200**；kill + 重启后 `GET`=**404**。**关键**：`MEMORY_DIR` 下 `54f1e626-….jsonl`、`ca4c6ddd-….jsonl` **确实已落盘** → 是「写了不读」：`get_session_status` 只查内存 map，`load_persisted_session` 是死代码未接线（routes.rs:149-162, session.rs:136-144）；误导测试 `test_p5_session_survives_restart` 只测 `store.load` 不测路由 |
| **N10** | **CLI sessions 契约断裂** | 🔴 | **已确认**【动态】 | `GET /api/v1/sessions` → **405**（`Allow: POST`）。服务端只注册 `post(create_session)`（main.rs:705），而 `codex-cli sessions` 子命令走 `GET`（main.rs:249 → client.rs:87）→ **子命令恒失败**。这是继 RT4/N1 之后的第二处 CLI↔服务端契约断裂，说明 CLI 无端到端契约测试 |
| **N11** | **坏 MEMORY_DIR 致进程崩溃** | 🔴 | **已确认**【动态】 | `MEMORY_DIR=/root/xx_unread/sub`（不可写）启动 → `thread 'main' panicked at crates/service/src/main.rs:503:10: failed to init civilization store: Permission denied (os error 13)`，进程直接退出。根因：`CivilizationStore::new(..).expect(..)` 把**可恢复的 I/O 错误升级为 panic**，无降级、无友好报错、无退出码语义 |

---

## C 组：契约与测试自身

| # | 场景 | 严重度 | 结果 | 证据 |
|---|---|---|---|---|
| CT1 | wiring 破坏测试 | 🔴 | **已确认**【动态】 | VM 上先构建 `codex-xray`，取基线 `wiring` **exit=0**；再把 `crates/agent-core/src/loop.rs` 中 1 处 `self.record_tool_exchange()` 调用整行注释掉（子串仍留在注释里）→ `wiring` **仍 exit=0**。纯子串 `content.contains` 匹配（project-xray/src/wiring.rs:112-158, main.rs:83-86），注释/死代码即可骗过断言；阈值过松（≥7 capabilities 即可绿）；spec 文件不受保护。<br>⚠️ **首轮探针在此处给出过假阳性**：`codex-xray` 当时未构建，而 `cmd \| tail -3; echo EXIT=$?` 取的是 `tail` 的退出码（恒 0）——结论对，证据链是错的，已重测 |
| CT2 | 测试自身验证 | 🟡 | 【静态】 | 需实际禁用一个测试验证套件变红；框架断言机制本身合理，待执行 |
| CT3 | 文档数字复验 | 🔵 | **已确认**【静态】 | `version-iteration-manual.md` 自相矛盾：摘要(行5/39)称 214 passed / wiring 15/15，v22.0 详节(行233)称 205 test / wiring 14/14 |
| CT4 | Docker 复验 | — | 【已验证·v22】 | v22 runbook 已复跑 Docker build 成功 + 容器 healthz/readyz 200；`_docker_verify.sh` 实为 codex-rust 脚本误置于 window-framework，Linux-only 且有无穷等待循环，勿用 |
| CT5 | version-iteration-manual v22 状态 | — | **安全**【静态】 | v22.0 正确标记 ✅ 已发布 / tag v22.0 / HEAD 8e39775（version-iteration-manual.md:5,39,228） |
| CT6 | 4 项旧债标记「已做」 | 🔵 | **已确认**【静态】 | `acceptance-final-epic-bc.md` 把 EPIC-B/C 旧债标 done，但：① civ 仍静默丢弃（RT2）矛盾；② /readyz 测试把弱点写成预期（RT1）矛盾；③ `quarterly-baseline-q3-2026.md` **仍不存在**（验收过度声称） |
| CT7 | wiring 阈值 + spec 保护 | 🔴 | 【静态】 | `xray_test.rs:38` 只要求 `>=7` 条 capability（实际 15），删 8 条仍绿；无断言锁 `wiring-v13.toml` 内容/哈希 |
| CT8 | window-framework 回归命令 | 🔵 | **已确认**【静态】 | README.md:153 全量循环 `for t in v07 v06 v05 v04 v03 v02 framework` 漏跑 `test_v09.py`+`test_v10.py`（共 38 断言）→ 177/177 无法按文档复现 |

---

## D 组：代码实测新增（最高杠杆，已并入上表对应行）

| 新发现 | 映射 | 严重度 | 证据 |
|---|---|---|---|
| CLI approve/deny 完全不可用（422 missing field `decision`） | RT4/N1 | 🔴 | 动态 |
| 未鉴权即可建会话并提交审批（无 401/403） | RT4/N2 | 🔴 | 动态 |
| `gate --reject` 装饰性（引擎只解析 `done:`，永不读 `blocked:`） | WT5/N3 | 🟡 | 静态 |
| stale-working 假完成（窗口从未运行却标 done） | WT6/N4 | 🔴 | 动态 |
| `bash` 工具零沙箱：`subprocess.run(cmd, shell=True)` 无白名单、无路径约束，`_allowed_write` 只管 `write_file` 管不到 bash | WT1/N5 | 🔴 | 静态（framework.py:491-495） |
| prompt 注入破坏 TOML（含 `\n` 的 prompt 让窗口永久不可读） | WT17/N6 | 🔴 | 动态 |
| **[表述已修正]** provider 路由只覆盖 agent 主循环；`_llm_summary`(1066-1088)、`_fc_analyze`(1452-1478)、`_yaml_analyze_fallback`(1481-1500) **三处硬编码 `DEEPSEEK_API_KEY` + `https://api.deepseek.com/v1`**。后果：① 窗口声明 `provider="zhipu"` 时 compress/analyze 仍强制走 deepseek，只配了 ZHIPU_API_KEY 则直接 `RuntimeError`；② 第 1074 行把 window.toml 的 `budget.model`（可能是 `glm-4.5`）发到 deepseek 端点 → 400；③ 若运维把非 deepseek 凭据填进 `DEEPSEEK_API_KEY` 当通用 key，**该凭据会被发往 api.deepseek.com** | N7 | 🔴 | 静态 |
| 重启后会话全 404（**数据已落盘，读路径未接线**）+ 误导测试 | RT9/N8 | 🔴 | 动态 |
| 并发限流全局 static 计数器（单客户端可吃满全局额度，无 Retry-After） | RT6/N9 | 🟡 | 动态 |
| **`codex-cli sessions` 子命令契约断裂（GET → 405）** | N10 | 🔴 | 动态 |
| **坏 `MEMORY_DIR` 触发 `.expect()` panic，进程退出** | N11 | 🔴 | 动态 |

---

## 验收判定

- **测试有效 ✅**：发现 🔴 14 项（远超「≥1」门槛）。红队纪律达成——不是「没漏洞」，是「打中了」。
- codex-rust 侧 **7 项动态项全部复现成功，0 项停留在推测**；RT8（回放）是唯一被判「安全」的动态项。
- 全部 🔴 均需开发者修复；🟡 为契约/边界不符；🔵 为文档失真（需同步修正文档，勿让文档虚假达标）。

### 元教训：审计工具自身必须被审计

首轮 VM 探针 6 项里有 **4 项因探针自身 bug 得出 INCONCLUSIVE、1 项得出假阳性**：

| 探针 bug | 后果 |
|---|---|
| `env -u VAR (subshell)` —— `env` 不能接子 shell，bash 直接 rc=2 | 服务从未启动，4 项全 HTTP 000 |
| GNU `env` 要求选项在 `VAR=值` **之前** | 第二轮 RT1/RT8 仍未启动 |
| `... & done; wait \| sort \| uniq -c` —— 管道接的是 `wait` 不是 curl | 429 计数恒 0 |
| `cmd \| tail -3; echo EXIT=$?` —— 取的是 `tail` 的退出码 | CT1 **假阳性**（且二进制根本没构建） |
| RT4 判据要求 200 才算「无鉴权」 | 实际 404 才是正确证据（请求已穿过鉴权层） |

**若不复核探针本身，这份报告会同时包含「假阴性」和「假阳性」。**
这正是本项目「不信报告信源码 / 测试必须能失败」铁律的又一次自证。

## 建议修复优先级（详见 `docs/audit-fix-taskbook-v22.md`）

1. **P0**：RT4（审批鉴权 + CLI 字段对齐 `decision`）、N10（CLI sessions 契约）、WT6/N4（working 恢复逻辑）、CT1（wiring 改 AST/语义匹配 + 锁 spec 哈希）、WT17/N6（prompt 净化 + TOML 安全写入）、WT1/N5（bash 工具沙箱）。
2. **P1**：RT9（会话持久化读路径接线）、RT1（就绪探针真实探活）、N11（启动错误降级不 panic）、RT2（civ_writer 缺失时落盘或显式告警）、RT3（seccomp 扩拒 socket/execve）、N7（compress/analyze 走 provider 路由）。
3. **P2**：WT3/WT7/WT13/WT14/WT15 边界硬化；RT6（per-client 限流 + Retry-After）；CT3/CT6/CT8 文档失真修正。

## 证据文件

- 本地：`.workbuddy/window_audit_evidence.jsonl`（window-framework 9 项动态）
- VM：`~/codex_work/v22_vm_evidence4.jsonl`（codex-rust 动态，**以此为准**）
- 探针脚本（可复跑）：
  - `.workbuddy/window_audit_probe.py` —— window-framework 本地探针
  - `.workbuddy/v22_audit_vm4.py` —— codex-rust VM 探针**终版**
  - `.workbuddy/v22_audit_vm.py` / `vm2` / `vm3` —— 前三轮，**结论已作废**，仅留作探针 bug 的复盘材料
- 运行日志：`.workbuddy/v22_vm4.log`（终版）、`v22_vm_local.log` / `v22_vm2.log` / `v22_vm3.log`（历史）
