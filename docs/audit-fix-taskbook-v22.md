# v22 红队审计 · 修复任务书

> 依据：`docs/audit-findings-v22.md`（v2 修订版）
> 基线（`git log` 实测，非凭记忆）：本地 HEAD **1255498**；tag **v22.0 → 8e39775**；
> window-framework `v1.0.3`；codex-cli `v22.0`。
> VM 上被测代码为上传快照，与本地 HEAD 同源。
> 日期：2026-08-02
> 被测环境：VM `192.168.220.131:~/codex_work`（Linux 真环境）+ 本地 window-framework

---

## 0. 使用说明

**给施工角色的三条硬约束**（依据项目铁律「不信报告信源码」「测试必须能失败」）：

1. **每条任务的「验收判据」必须先写成一个会失败的测试，再改代码让它变绿。**
   先跑一遍确认新测试在**未修复的代码上是红的** —— 否则这个测试没有牙齿，等于没写。
2. **不得只改测试让它过。** 生产路径必须真接线；本次审计中 RT1/RT9 就是「测试绿、生产死」的典型
   （单测打的是 mock，生产走的是另一条分支）。
3. **动手前复核本文的 file:line。** 本文行号基于上述基线 commit，若已 rebase 请以源码为准。

**优先级定义**

| 级别 | 含义 | 处置 |
|---|---|---|
| **P0** | 安全边界失效 / 核心功能完全不可用 / 质量门禁可被绕过 | 阻塞发布，必须修 |
| **P1** | 可用性或可观测性缺陷，有明确故障场景 | 本轮内修 |
| **P2** | 边界硬化、体验、文档失真 | 排期修，可批量 |

**任务总览**：P0 共 6 条，P1 共 6 条，P2 共 8 条，合计 **20 条**。

---

# P0 —— 阻塞发布

## P0-1　审批门默认零鉴权

- **组件**：codex-rust / service
- **定位**：`crates/service/src/routes.rs:68-103`（`require_api_key`）、`crates/service/src/main.rs:526-545`（`API_KEY` 读取）、`main.rs:762-768`（中间件层序）
- **复现（动态，已实测）**：VM 上不设 `API_KEY` 启动服务后，
  - 零凭据 `POST /api/v1/sessions` → **201 Created**，拿到真实 `session_id`；
  - 零凭据 `POST /api/v1/sessions/{sid}/approvals` → **404 SESSION_NOT_FOUND**（**不是 401/403**）；
  - 带 `Authorization: Bearer totally-wrong-key` → 结果**完全相同**。
  - 结论：请求穿过鉴权中间件直达业务处理器，鉴权层在默认配置下形同不存在。
- **根因**：`API_KEY` 未设置时中间件选择「全放行」而非「全拒绝」；启动日志只打一条
  `WARN: API is OPEN (anyone reaching this port can run commands via the agent)` 就继续服务。
  **默认不安全（insecure by default）** —— 而这个服务的能力是「代执行 shell 命令」。
- **修复方向**：
  1. 反转默认：未设置 `API_KEY` 时**默认拒绝**所有 `/api/v1/*`，仅放行 `/healthz`、`/readyz`。
  2. 若要保留本机开发便利，改为**显式选择加入**：`ALLOW_NO_AUTH=1` 才放行，且此时**只绑定 `127.0.0.1`**
     （当前绑 `0.0.0.0:3000`，见启动日志 `Codex Agent Service starting on 0.0.0.0:3000`，
     这意味着无鉴权服务直接暴露在局域网上）。
  3. 鉴权失败必须返回 **401**（缺凭据）/ **403**（凭据错误），不得让请求落到业务处理器。
- **验收判据**：
  - [ ] 新增集成测试：不设 `API_KEY` 启动 → `POST /api/v1/sessions` 断言 **401**（当前会拿到 201，测试必须先红）。
  - [ ] 新增测试：设 `ALLOW_NO_AUTH=1` 时监听地址断言为 `127.0.0.1`，不得为 `0.0.0.0`。
  - [ ] 新增测试：设 `API_KEY=k` 后，带错误 key → **403**；带正确 key → 200/201。
  - [ ] `/healthz`、`/readyz` 在任何配置下均不要求鉴权（回归保护）。

## P0-2　codex-cli `approve` / `deny` 子命令完全不可用

- **组件**：codex-cli ↔ service（契约断裂）
- **定位**：`crates/api/src/lib.rs:69-72`（`ApprovalReq { approval_id, decision: String }`）
  vs `crates/codex-cli/src/client.rs:135-138`（实发 `{"approval_id", "approved": bool}`）；
  路由 `crates/service/src/routes.rs:181-194`
- **复现（动态，已实测）**：照 CLI 真实报文发包 →
  **`422`**，服务端原文：``Failed to deserialize the JSON body into the target type: missing field `decision` at line 1 column 42``
- **影响**：`codex-cli approve` / `codex-cli deny`（main.rs:268、273）**在任何情况下都无法工作**。
  这意味着「人工审批」这条安全链路在 CLI 侧是断的 —— 与 P0-1 叠加，等于审批机制整体失效。
- **修复方向**：以服务端 `ApprovalReq` 为准，改 CLI 发 `{"approval_id": <id>, "decision": "approve"|"deny"}`。
  （不要反向改服务端 —— `decision:String` 比 `approved:bool` 更可扩展，能表达 `defer`/`abort`。）
- **验收判据**：
  - [ ] 新增**端到端契约测试**：起真实 service → `codex-cli approve <sid> <aid>` → 断言非 4xx。当前必然红。
  - [ ] 新增序列化测试：`ApprovalReq` 的 CLI 侧构造体与 `api` crate 的定义**共用同一个类型**
        （建议 CLI 直接 `use api::ApprovalReq`，从类型层面消除漂移，而不是靠测试追）。

## P0-3　codex-cli `sessions` 子命令契约断裂（GET → 405）

- **组件**：codex-cli ↔ service
- **定位**：`crates/service/src/main.rs:705`（只注册 `post(routes::create_session)`）
  vs `crates/codex-cli/src/client.rs:87`（`GET /api/v1/sessions`）← `crates/codex-cli/src/main.rs:249`
- **复现（动态，已实测）**：`GET /api/v1/sessions` → **405**，响应头 `Allow: POST`。
- **影响**：`codex-cli sessions` 恒失败。与 P0-2 同源 —— **CLI 与服务端之间没有任何端到端契约测试**，
  两处断裂都能带着 214 个通过的单测发布出去。
- **修复方向**：
  1. 服务端补 `GET /api/v1/sessions` → 返回 `{"sessions":[...]}`（CLI 已按此结构解析，client.rs:93）。
     数据源用 `list_persisted_sessions()` + 内存活动会话合并。
  2. **顺带解决 P1-1 的一半** —— 这个列表接口天然需要读持久化层。
- **验收判据**：
  - [ ] 新增测试：`GET /api/v1/sessions` 返回 200 且 body 含 `sessions` 数组（当前 405，先红）。
  - [ ] 新增端到端：建 2 个会话 → `codex-cli sessions` 输出含这 2 个 id。
  - [ ] **补一个覆盖全部 CLI 子命令的 smoke 契约测试**（见 P1-6），防止第三次出现同类断裂。

## P0-4　wiring 断言可被「注释掉调用」绕过

- **组件**：project-xray（质量门禁本身）
- **定位**：`crates/project-xray/src/wiring.rs:112-158`（`ChainLink` 用 `content.contains(literal)`）、
  `crates/project-xray/src/main.rs:83-86`；阈值 `crates/project-xray/tests/xray_test.rs:38`（`>=7`）
- **复现（动态，已实测）**：VM 上构建 `codex-xray` → 基线 `wiring` **exit=0**；
  把 `crates/agent-core/src/loop.rs` 中 1 处 `self.record_tool_exchange();` **整行注释掉**
  （子串仍留在注释里）→ `wiring` **仍 exit=0**。
- **影响**：这是**门禁自身失效**，比任何单个业务缺陷更严重 —— 它让所有 G1 级「必执行」断言退化为
  「源码里出现过这个字符串即可」。按项目基因定义（`docs/gene-expression-system-design.md`），
  G1 的强度承诺是「代码分支必执行」，当前实现只能兑现「文本存在」。
- **修复方向**（按投入递增，建议至少做到 2）：
  1. **最小止血**：匹配前剥离注释与字符串字面量（行注释 `//`、块注释 `/* */`、`r"..."`）。
  2. **推荐**：改用 `syn` 解析 AST，断言「在函数 X 的**可达语句**中存在对 Y 的调用」。
     ⚠️ 依赖现状已核实：`syn` 目前只是**传递依赖**（`Cargo.lock` 中存在 `syn 2.0.119` 与 `syn 3.0.3`
     两个版本，由 proc-macro 类 crate 间接引入），**任何 `Cargo.toml` 里都没有直接声明**
     （`grep -rn '^syn' Cargo.toml crates/*/Cargo.toml` 无命中）。因此需要给 `project-xray`
     **新增一条直接依赖**（建议 `syn = { version = "2", features = ["full", "visit"] }`，
     与已在 lock 中的 2.0.119 对齐，避免把 3.x 拖进主依赖树）。
     好处：不引入编译期网络请求（离线 vendor 可解），符合 xray 设计铁律 C.6。
  3. **锁 spec**：把 `wiring-v13.toml` 的 SHA-256 写进测试断言，防止「改断言让门禁变绿」。
- **验收判据**：
  - [ ] 新增**破坏性测试**：测试内临时注释掉一处被断言的调用 → 断言 `wiring` 返回**非 0**。当前必然红。
  - [ ] 新增测试：把调用改成死代码（`if false { ... }`）→ 断言返回非 0（进阶，做了方案 2 才能过）。
  - [ ] `xray_test.rs:38` 阈值从 `>=7` 改为**精确等于当前 capability 条数**，并新增
        「删任意 1 条 capability → 测试变红」的断言。
  - [ ] 新增 `wiring-v13.toml` 内容哈希断言。

## P0-5　window-framework `bash` 工具零沙箱

- **组件**：window-framework
- **定位**：`window-framework/src/framework.py:491-495`
  ```python
  if name == "bash":
      cp = subprocess.run(args["cmd"], shell=True, capture_output=True,
                          text=True, cwd=str(self.root).replace("\\", "/"), timeout=30)
  ```
- **根因**：`_allowed_write`（framework.py:416-422）只约束 `write_file` 工具，**完全约束不到 `bash`**。
  LLM 只要选择 `bash` 而不是 `write_file`，就能以框架进程的完整权限执行任意命令 ——
  写任意路径、读任意文件、发起网络请求、`rm -rf`。沙箱设计存在但可被绕行。
- **附带**：`_allowed_write` 自身还有前缀旁路缺陷（`str.startswith` 无分隔符边界，
  `outputs_evil/`、`outputsX/` 均被放行）—— 见 P1-5。
- **修复方向**：
  1. 去掉 `shell=True`，改 `shlex.split` + 命令**白名单**（`ls/cat/grep/find/python/pytest/git status` 等只读或受限命令）。
  2. 所有路径参数经 `_allowed_read`/`_allowed_write` 校验后才允许。
  3. 禁网：清空子进程 env 中的代理变量，或在容器/`unshare -n` 中执行。
  4. 若判断白名单不现实，**至少**加一个显式的 `WINDOW_ALLOW_RAW_BASH=1` 开关，默认关闭。
- **验收判据**：
  - [ ] 新增测试：`bash` 工具执行 `echo x > /tmp/escape_probe` → 断言**被拒绝**且 `/tmp/escape_probe` 不存在。当前必然红。
  - [ ] 新增测试：`bash` 执行 `curl http://example.com` → 断言被拒绝。
  - [ ] 新增测试：`bash` 执行白名单内的 `ls outputs` → 断言正常返回（防止修过头）。

## P0-6　prompt 注入破坏 window.toml（窗口永久不可读）

- **组件**：window-framework
- **定位**：`window-framework/src/framework.py:35-63`（TOML 模板）、`:222`（`window create` 插值点）
- **复现（动态，已实测）**：`window create` 时传入含 `\n` 的 prompt → 模板插值未转义换行/反斜杠 →
  生成的 `window.toml` 语法损坏 → 之后任何操作 `tomllib.loads` 抛异常，**该窗口永久不可打开**。
- **影响**：不仅是可用性（窗口砖化），还是**注入面** —— 精心构造的 prompt 可以注入新的 TOML 键值，
  例如覆写 `[budget] provider` 或 `max_steps`。
- **修复方向**：
  1. **不要用字符串模板拼 TOML。** 用 `tomli_w`（或手写 escape 函数）序列化 dict。
  2. 若必须保留模板，对所有插值做 TOML 基本字符串转义（`\` `"` `\n` `\r` `\t`）。
  3. `window create` 后立即 `tomllib.loads` 自检，失败则回滚创建。
- **验收判据**：
  - [ ] 新增测试：prompt = `"line1\nline2"` → 创建成功且 `tomllib.loads` 能正常解析回原值。当前必然红。
  - [ ] 新增测试：prompt = `'x"\n[budget]\nmax_steps = 99999\n'` → 断言解析后的 `budget.max_steps`
        **仍是原值**（注入未生效）。
  - [ ] 新增测试：prompt 含 `\` 与 `"` → 往返一致。

---

# P1 —— 本轮内修复

## P1-1　重启后会话全部 404（数据已落盘，读路径未接线）

- **组件**：codex-rust / service
- **定位**：`crates/service/src/routes.rs:149-162`（`get_session_status` 只查内存 map）、
  `crates/service/src/session.rs:136-144`（`load_persisted_session` —— **死代码，无生产调用点**）
- **复现（动态，已实测）**：建会话 → `GET /api/v1/sessions/{id}` = **200** → kill 进程 → 重启 →
  同一 `GET` = **404**。**关键证据**：`MEMORY_DIR` 目录下 `54f1e626-….jsonl`、`ca4c6ddd-….jsonl`
  **确实已经写在磁盘上** —— 所以这不是「没持久化」，是**写了不读**。
- **误导测试**：`test_p5_session_survives_restart` 只调用 `store.load(...)` 断言数据还在，
  **完全没有经过 HTTP 路由** —— 测试名承诺的是「会话能扛住重启」，实际验证的是「文件还在」。
  这是本次审计里第二个「测试绿、生产死」的样本（另一个是 P1-2）。
- **修复方向**：`get_session_status` 内存 miss 时回落到 `load_persisted_session`，命中则重建
  `SessionStatus` 返回 200；确实不存在才 404。
- **验收判据**：
  - [ ] **重写** `test_p5_session_survives_restart`：必须经 HTTP 路由 —— 建会话 → 重启 → `GET` 断言 **200**。当前必然红。
  - [ ] 保留原「文件仍在」的断言，但改名为 `test_p5_session_record_persisted_to_disk`，不要再用
        `survives_restart` 这种**承诺大于验证**的命名。
  - [ ] 新增：不存在的 id → 仍返回 404（防止修过头把 404 也吃掉）。

## P1-2　`/readyz` 就绪探针永远不会变红

- **组件**：codex-rust / service + memory
- **定位**：`crates/memory/src/lib.rs:222-233`（`list_sessions` 用 `if let Ok(entries) = std::fs::read_dir(...)`
  **吞掉全部 I/O 错**，恒返 `Ok`）、`crates/service/src/session.rs:128-133`（无 store 时返回 `Ok(vec![])`）、
  `crates/service/src/routes.rs:257-265`（503 分支）
- **复现（动态，已实测）**：服务正常启动后把 `MEMORY_DIR` `chmod 000`（`ls` 已报「权限不够」）→
  `/readyz` **仍返回 200**，日志中 `session store unavailable` 出现 **0 次**。
- **影响**：503 分支是**生产不可达的死代码**。K8s/负载均衡拿这个探针做流量准入，
  会把请求打给一个存储已经坏掉的实例 —— 就绪探针给了**虚假的绿**。
- **误导测试**：`crates/service/tests/integration_test.rs:1996-2024` 用一个恒返 `Err` 的
  `FailingStore` mock 覆盖 503 分支。mock 能失败，**生产实现不能失败** —— 测试覆盖的是一条
  生产永不会走的路径。
- **修复方向**：
  1. `list_sessions` 把 `read_dir` 的错误**向上传播**（`std::fs::read_dir(&self.dir)?`），不要吞。
  2. `readyz` 增加一次**写探活**（在 store 目录写一个临时文件再删），只读成功不足以证明就绪。
  3. `memory_store` 为 `None` 时应返回 503 而不是 `Ok(vec![])` —— 「没有存储」不等于「就绪」。
- **验收判据**：
  - [ ] 新增集成测试：**用真实 `JsonlMemoryStore`**（不是 mock）指向不可读目录 → 断言 `/readyz` = **503**。当前必然红。
  - [ ] 新增测试：`JsonlMemoryStore::list_sessions` 在目录不可读时返回 `Err`（当前返回 `Ok`）。
  - [ ] 保留正常情况 200 的回归断言。

## P1-3　坏 `MEMORY_DIR` 触发 panic，进程直接退出

- **组件**：codex-rust / service
- **定位**：`crates/service/src/main.rs:496-504`
  ```rust
  let civ_store = Arc::new(
      memory::CivilizationStore::new(&PathBuf::from(&memory_dir), "civilization.jsonl")
          .expect("failed to init civilization store"),   // ← main.rs:503
  );
  ```
- **复现（动态，已实测）**：`MEMORY_DIR=/root/xx_unread/sub`（不可写）启动 →
  `thread 'main' panicked at crates/service/src/main.rs:503:10: failed to init civilization store: Permission denied (os error 13)`，进程退出。
- **影响**：一个**可恢复的配置/权限错误**被升级为进程崩溃。运维拿到的是 panic 堆栈而不是
  「MEMORY_DIR 不可写，请检查权限」。容器环境下表现为 CrashLoopBackOff，排障成本高。
- **修复方向**：启动期所有 I/O 初始化改为「打印可操作的错误信息 + `std::process::exit(非0)`」，
  或降级为「civ 功能禁用但服务可启动」。**扫一遍 `main.rs` 里所有 `.expect(` / `.unwrap(`**，
  按同样标准处理（本条不止这一处）。
- **验收判据**：
  - [ ] 新增测试：`MEMORY_DIR` 指向不可写路径启动 → 断言进程**不 panic**，stderr 含明确提示，退出码非 0。当前必然红。
  - [ ] 静态检查：`crates/service/src/main.rs` 中启动路径的 `.expect(`/`.unwrap(` 计数写入断言（防回潮）。

## P1-4　civ 告警三层静默丢弃

- **组件**：codex-rust / agent-core + nervous-system
- **定位**：
  - `crates/agent-core/src/loop.rs:475-479`：`civ_note` 是 `if let Some(ref w) = self.civ_writer { ... }`
    —— **没有 `else`**。writer 未接线时告警**无声消失**。
  - `crates/agent-core/src/loop.rs:1665-1669`：`drain_civ_alerts()` **仅此一处调用点**，位于 `do_reflect`
    —— 未走到 reflect 相位的告警全部丢失。
  - `crates/nervous-system/src/lib.rs:83-85`：append I/O 失败仅 `warn!`。
- **影响**：神经系统「告警」这条链路在三个层次上都可以无声失败，且**没有任何一层会让测试变红**。
- **修复方向**：
  1. `civ_note` 补 `else` 分支：writer 未接线时至少 `tracing::warn!` 并落到本地文件。
  2. `drain_civ_alerts` 在每个相位结束都调用一次，或改由后台任务定时 drain。
  3. append 失败从 `warn!` 升级为「重试 + 计数指标」，连续失败暴露到 `/readyz`。
- **验收判据**：
  - [ ] 新增测试：`civ_writer = None` 时产生一条告警 → 断言 fallback 落盘文件存在。当前必然红。
  - [ ] 新增测试：在 `do_act` 相位产生告警且**不进入** `do_reflect` → 断言告警仍被 drain。
  - [ ] 新增测试：append 失败 N 次后 `/readyz` 返回 503。

## P1-5　沙箱路径前缀旁路 + 产出验证形同虚设

- **组件**：window-framework
- **定位**：
  - `framework.py:416-422`：`_allowed_write` 用 `str(p).startswith(str(d))`，**无路径分隔符边界** →
    `outputs_evil/x`、`outputsX/x` 全部被判为「在 outputs 目录内」。（动态已复现）
  - `framework.py:457-462`：`_has_outputs` **只检查目录非空** → 写一个 0 字节文件即算「有产出」。（动态已复现）
- **修复方向**：
  1. `_allowed_write` 改用 `Path.is_relative_to()`（Python 3.9+），或比较时补 `os.sep`。
  2. `_has_outputs` 至少要求存在**非空**文件；更好的做法是校验 stage 声明的产出清单。
- **验收判据**：
  - [ ] 新增测试：写 `outputs_evil/x.txt` → 断言**被拒**。当前必然红。
  - [ ] 新增测试：写 `outputs/../../etc/x` → 断言被拒。
  - [ ] 新增测试：outputs 下只有 0 字节文件 → `_has_outputs` 返回 **False**。当前必然红。

## P1-6　CLI ↔ service 缺少端到端契约测试（P0-2 / P0-3 的共同根因）

- **组件**：测试体系
- **背景**：本次审计在 CLI 侧一口气发现 **2 处**契约断裂（`approve/deny` 422、`sessions` 405），
  而 codex-rust 有 214 个通过的测试。原因是这 214 个测试**没有一个真正跑过 CLI 二进制打真服务**。
- **修复方向**：新增一个 smoke 契约测试套件，对 `codex-cli` **每一个子命令**
  （`main.rs` 中 `Commands::` 分支，实测 `grep -cE '^\s+Commands::' crates/codex-cli/src/main.rs` = **17 个**）
  起真实 service 各打一次，断言返回非 4xx/5xx（业务失败可接受，**协议层失败不可接受**）。
  这是一次性投入，能挡住整类问题。
- **验收判据**：
  - [ ] 新套件覆盖全部子命令，当前应至少 **2 条红**（approve/deny、sessions）。
  - [ ] 修完 P0-2 / P0-3 后全绿。
  - [ ] 接入 CI 门禁。

---

# P2 —— 排期修复

## P2-1　并发限流无 per-client 维度

- **定位**：`crates/service/src/routes.rs:271-284`（`static P1_CONCURRENT_COUNT: AtomicUsize`，`MAX=50`）
- **复现（动态，已实测）**：单客户端 60 条 `--limit-rate 5k` 慢速大 body 上传 → **50×201 + 10×429**。
  限流器确实在第 50 条在途时触发（机制有效），但计数器是**全局**的，无 IP/用户维度 →
  单个客户端即可吃满全局额度，被拒的第 51 条同样可能是他人的请求。
- **修复**：改 per-IP/per-token 令牌桶；429 补 `Retry-After` 响应头；健康探针端点豁免限流
  （当前 `/healthz`、`/readyz` 绕过鉴权但**不绕过限流**，见 `main.rs:762-768` 层序）。
- **验收**：[ ] 客户端 A 打满额度后，客户端 B 的请求仍应成功（当前会被拒）。[ ] 429 响应含 `Retry-After`。
- ⚠️ **门禁覆盖范围**（`v22_regression_gate.py` 的 `P2-1` 项）：**只自动验「429 是否带 Retry-After」**
  （已复现 10/10 不带）。另两点**动态测不出，属人工审查项**：
  ① per-client 维度 —— VM 只有单 IP，造不出双客户端；
  ② 健康端点豁免 —— `--limit-rate` 只限客户端发送速率，300KB body 被 localhost 内核缓冲一次吃下，
  handler 秒返，限流饱和窗口仅毫秒级，探测必然落空（5k/2k 两种速率均已试过）。
  **验收此项时必须人工读 `main.rs:768` 与 `routes.rs:275-284` 确认，不得只看门禁绿灯。**

## P2-2　seccomp 拒绝清单过窄

- **定位**：`crates/sandbox/src/lib.rs:391-405`，精确拒绝 6 个 syscall：
  `ptrace(101) / reboot(169) / init_module(175) / delete_module(176) / kexec_load(246) / kexec_file_load(320)`
- **未拦截**：`socket` / `connect`（**网络外泄**）、`execve`、`clone` / `unshare`；
  第 390 行注释明确写着 `mount/umount2 are NOT denied`。
- **附带**：landlock 应用失败仅 `warn!` 继续执行（lib.rs:294-375）；非 Linux 平台走 `NoopSandbox`，**零隔离**（lib.rs:112-130）。
- **修复**：改为**白名单**（默认拒绝）；至少补 `socket`/`connect` 拒绝，并让 landlock 失败变成**硬失败**。
- **验收**：[ ] 沙箱内 `curl` / `socket()` 断言被拒。[ ] landlock 不可用时断言拒绝启动而非降级。

## P2-3　`gate --reject` 是装饰

- **定位**：`framework.py:836`、`:995` —— 引擎只解析 `done:`，**永不读 `blocked:`**。
- **修复**：`_run_gate` 识别 `blocked:` 并阻断 stage 推进；approve 改为幂等。
- **验收**：[ ] `--reject` 后 stage 断言**不**推进（当前会推进）。[ ] 重复 approve 结果一致。

## P2-4　stale-`working` 假完成

- **定位**：`framework.py:857-883` —— 陈旧 `working` 重置为 pending 后 `continue`，
  **跳过了 `all_done = False`**（877-883）→ 自动 gate `echo VERIFY_PASS` 直接把 stage 标 done，
  **窗口从未运行**。（动态已复现）
- **修复**：`continue` 前先置 `all_done = False`。**一行修复，但后果是整个 stage 假完成** —— 优先做。
- **验收**：[ ] 构造陈旧 `working` 窗口 → 断言 stage **不**被标 done。当前必然红。

> ⚠️ 本条虽列在 P2 区块内（属 window-framework 边界类），但修复成本极低、影响是「工作流假完成」，
> **建议与 P0 同批处理**。

## P2-5　compress / analyze 绕过 provider 路由

- **定位**：`framework.py:1066-1088`（`_llm_summary`）、`:1452-1478`（`_fc_analyze`）、
  `:1481-1500`（`_yaml_analyze_fallback`）—— 三处硬编码 `DEEPSEEK_API_KEY` +
  `https://api.deepseek.com/v1`，绕过 `PROVIDERS` 路由表（`:385-414`）。
- **后果**：① 窗口声明 `provider="zhipu"` 时 compress/analyze 仍强制走 deepseek，只配了
  `ZHIPU_API_KEY` 会直接 `RuntimeError`；② 第 **1074** 行把 window.toml 的 `budget.model`
  （可能是 `glm-4.5`）发到 deepseek 端点 → 400；③ 若运维把非 deepseek 凭据填进 `DEEPSEEK_API_KEY`
  当通用 key，**该凭据会被发往 api.deepseek.com**。
- **附带**：`PROVIDERS.get(provider, PROVIDERS["deepseek"])`（`:411`）—— 未知 provider **静默回退** deepseek，
  拼写错误不报错。
- **修复**：三处改走 `PROVIDERS` 路由；未知 provider 改为**显式报错**而非静默回退。
- **验收**：[ ] `provider="zhipu"` 且只设 `ZHIPU_API_KEY` → compress 走智谱端点成功。当前必然红。
  [ ] `provider="typo"` → 断言**抛错**，不得静默走 deepseek。

## P2-6　快照 / 回滚可致净数据丢失

- **定位**：`framework.py:707-746` —— 快照只含 2 个文件（**不含 outputs**）；
  回滚**先 move 当前对话再判断**目标是否有效 → 目标无效时当前对话已经没了；`--to` 参数存在路径遍历。
- **修复**：快照纳入 outputs；回滚改为「先校验目标 → 备份当前 → 再替换」；`--to` 做白名单校验。
- **验收**：[ ] 回滚到不存在的快照 → 断言当前对话**完好无损**。当前必然红。[ ] `--to ../../x` 被拒。

## P2-7　其他边界硬化（可批量）

| 项 | 定位 | 修复要点 | 验收 |
|---|---|---|---|
| WT7 analyze 缺 `id` 崩溃 | `framework.py:1356`（`w["id"]` 下标先于 1376 的字段检查） | 校验前置 + 捕获 | 缺 id 的 window → 友好报错不 KeyError |
| WT4 budget 极端值 | `framework.py:552-563`、`:535` | replay 模式也查预算；CLI 路径尊重 `budget.max_steps` | `max_steps=99999` 不产生 ~2000 次快照 |
| WT13 压缩质量退化 | `framework.py:1108-1153` | 连压收敛检测，停止反复摘要 | 连压 5 次行数序列不应卡在 `[31,23,21,21,21]` |
| WT15 导入导出有损 | `framework.py:1673-1733`、`:1692` | 导出不截断 200 字符；导入保留 `summary` role 与原时间戳 | 往返一致性断言 |
| WT11 gate 路径遍历 | `framework.py:2122-2128`（`gate_spec[5:]` 无校验） | 路径白名单 | `auto:../../x` 被拒 |
| WT19 重复 role 不查 | `framework.py:79-134`、`:212-229` | `framework check` / `window create` 补查重 | 重复 role 断言报错 |

## P2-8　文档失真修正

| 项 | 定位 | 问题 | 修复 |
|---|---|---|---|
| CT3 | `docs/version-iteration-manual.md:5,39` vs `:233` | 摘要称 `214 passed / wiring 15-15`，v22.0 详节称 `205 test / wiring 14-14` —— **自相矛盾** | 以实测 214/15-15 为准统一；补 v22 → 窗口群框架段（当前 §三 275 行仍误写「窗口群框架未工程化」） |
| CT6 | `acceptance-final-epic-bc.md` | 把 EPIC-B/C 旧债标 `done`，但：① civ 仍静默丢弃（P1-4）矛盾；② `/readyz` 测试把弱点写成预期（P1-2）矛盾；③ **`quarterly-baseline-q3-2026.md` 至今不存在** | 撤销过度声称；未完成项改回 open |
| CT8 | `window-framework/README.md:153` | 全量回归循环 `for t in v07 v06 v05 v04 v03 v02 framework` **漏 `test_v09.py` + `test_v10.py`**（共 38 断言）→ 文档写的 177/177 按文档命令**复现不出来** | 补齐两个测试文件 |

---

## 附录 A　修复顺序建议

```
第 1 批（阻塞发布，安全 + 门禁）
  P0-1 审批鉴权  →  P0-2 CLI decision 字段  →  P0-3 GET /sessions
  P0-4 wiring AST 匹配（门禁先修，否则后续修复无法被门禁保护）
  P0-5 bash 沙箱  →  P0-6 TOML 注入
  P2-4 stale-working（一行修复，但后果是假完成，随批带走）

第 2 批（可用性 + 可观测性）
  P1-1 会话读路径  →  P1-2 readyz 真探活  →  P1-3 启动不 panic
  P1-4 civ 告警  →  P1-5 路径边界  →  P1-6 CLI 契约测试套件

第 3 批（硬化 + 文档）
  P2-1 ~ P2-3、P2-5 ~ P2-8
```

## 附录 B　本次审计暴露的三个系统性问题

修单条缺陷之外，更值得处理的是这三个**成因**：

1. **测试打 mock，生产走别的分支**（P1-1 `survives_restart` 只测 store.load、
   P1-2 `readyz` 只测 FailingStore）。
   → 建议：凡是名字里承诺「端到端 / survives / 探活」的测试，必须经过真实的入口（HTTP 路由 / CLI 二进制）。
2. **CLI 与服务端之间零契约测试**（P0-2、P0-3 两处断裂并存）。
   → 建议：P1-6 的 smoke 套件 + CLI 直接复用 `api` crate 的请求类型，从类型层面消除漂移。
3. **门禁只验「文本存在」不验「行为发生」**（P0-4）。
   → 这与项目自身的三层递进方法论（存在性 → 行为 → 表达力）冲突：
     `codex-xray wiring` 目前停在第 ① 层，却被当作第 ② 层的证据使用。

## 附录 C　复跑命令

```bash
# window-framework 本地动态探针（零成本）
cd C:/Users/87465/Desktop/codex-rust-v1.0-final/.workbuddy
python window_audit_probe.py          # → window_audit_evidence.jsonl

# codex-rust VM 动态探针（终版，零 token）
CODEX_VM_PW=123456 python v22_audit_vm4.py   # → ~/codex_work/v22_vm_evidence4.jsonl

# 回放回归（VM，零 token，当前 31/31）
ssh wutao@192.168.220.131
cd ~/codex_work && source ~/.cargo/env
(setsid env -u API_KEY REPLAY_DIR=$PWD/bench/replay/fixtures ./target/release/service >~/service.log 2>&1 &)
python3 bench/replay.py
```

> ⚠️ 探针脚本 `v22_audit_vm.py` / `vm2` / `vm3` 的**结论已作废**（探针自身有 bug），
> 保留仅供复盘，勿据其结论施工。
