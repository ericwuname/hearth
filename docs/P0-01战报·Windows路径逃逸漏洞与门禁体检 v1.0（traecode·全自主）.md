# P0-01 战报 · Windows 路径逃逸漏洞与门禁体检 v1.0

- **出品**：traecode　**日期**：2026-09-30　**授权**：用户全权（"全部自主、无需汇报"）
- **基线**：`p0-usability-01` @ `6871f7a`　**环境**：本机 Windows MSVC（`.131/.133` 不可达）
- **依据**：《Hearth 长程优化探索规划 v1.0》P0 阶段第一张卡
- **结论先行**：**发现并修复 1 个真实安全漏洞 + 2 个门禁引擎缺陷 + 1 处长期门禁欠账**。
  原战报把 3 个失败 target 一概归为"环境/既有"，**其中 8 项实为真实代码缺陷**。
  另有 1 项（3 条能力锁定冲突）**属改契约，已停手，待顶层裁决**。

---

## 一、结果总览

| 指标 | 修复前 | 修复后 |
|---|---|---|
| 失败 target | **3** | **1**（`project-xray`，余 3 条能力锁定冲突 → 待裁决） |
| 失败用例 | **13** | **3**（全部为待裁决项，非缺陷） |
| `tools-builtin` | 74 passed / **11 failed** | 见第九节门禁 |
| `sandbox` | 5 passed / **1 failed** | **全绿** |
| `project-xray` | 1 passed / **1 failed** | 1 passed / 1 failed（hash 锁已修，余 3 条待裁决） |
| 新增回归测试 | — | 1（UTF-8 锚点） |
| 发现的新缺陷（立卡） | — | 3（D-11 待裁决 / D-12、D-13 待修） |

**核心洞察**：原分类"环境/既有"掩盖了真问题。逐一核验后发现，**"测试红"里藏着真漏洞**——
这条经验对后续所有"既有红"都成立：**不得以"环境问题"结案，必须逐条取证**。

---

## 二、修复 1（最高优先级）· Windows 路径守卫失效 → 工作区逃逸【真实安全漏洞】

### 2.1 发现过程（非推理，是实证）

跑基线时见 `tools-builtin` 11 项红。其中多项是**路径穿越守卫测试**，与"POSIX 语义差"的直觉不符，遂专项核验。

**决定性证据**：检查文件系统发现 **`C:\etc\passwd` 真实存在**，1 字节，时间戳 `2026-09-30 18:16:07`
——正是基线测试跑批的时刻。**测试没有"断言失败"，是工具真的把文件写到了工作区之外。**

（该残留已由我删除；修复后复跑，`C:\etc` 不再产生。）

### 2.2 根因链

```rust
// crates/tools-builtin/src/lib.rs（修复前）
let p = std::path::Path::new(raw);
if !p.is_absolute() { return true; }   // ← Windows 下 /etc/passwd 在此直接放行
```

1. Windows 的"绝对路径"**必须带盘符前缀**，故 `Path::new("/etc/passwd").is_absolute()` == **false**；
2. 于是 `/etc/passwd` 被当作"相对路径"**跳过白名单**；
3. 而 `PathBuf::join` 在 Windows 上遇到**有根无前缀**的路径（`/x`、`\x`）会**替换掉盘符之后的一切**：

   `C:\...\crates\tools-builtin`.join(`/etc/passwd`) → **`C:\etc\passwd`**

**影响面**：同一模式出现在 **4 个工具**——`edit.rs`（write_file）、`read.rs`、`patch.rs`（apply_patch）、`grep.rs`。
即 Windows 上**写/读/检索三个方向都可越界**。

### 2.3 修复

新增集中判定纯函数，5 处调用点统一改判：

```rust
/// 判定"该路径被 join 到 cwd 时可能逃出工作区"。
/// 凡"有根"（has_root）或"带盘符前缀"（Component::Prefix）者，一律视作需白名单校验。
pub fn is_rooted_path(p: &std::path::Path) -> bool {
    p.has_root()
        || matches!(p.components().next(), Some(std::path::Component::Prefix(_)))
}
```

- 覆盖**两种逃逸形态**：①"有根无前缀"（`/etc/passwd`）②"盘符相对"（`C:foo`）；
- 纯相对文件名（`a.txt`、`test..txt`）仍返回 false，**不收窄正常用法**；
- 改动点：`lib.rs`（判定本体）+ `edit.rs:84` + `read.rs:82` + `patch.rs:84` + `grep.rs:103`。

### 2.4 先红后绿（有实证）

| | `tools-builtin` |
|---|---|
| 红（修复前） | 74 passed / **11 failed** |
| 绿（修复后） | **82 passed / 3 failed**（8 项转绿） |

**"8/11 转绿"本身就是漏洞存在的独立证据**——若这 8 项真是"环境问题"，改路径判定不可能让它们变绿。

---

## 三、修复 2 · xray 剥离器 UTF-8 缺陷（门禁假阴性）

`crates/project-xray/src/wiring.rs` 的 `strip_comments_and_strings`：

```rust
out.push(c as char);   // c 是 u8 —— 按"码点"转 char
```

`c` 是 UTF-8 的**单个字节**，`c as char` 会把多字节字符拆成逐字节乱码
（`编` = `E7 BC 96` → 3 个 Latin-1 字符）。**后果**：

- **ASCII 锚点照常通过；任何含中文的锚点永远匹配不上** → 门禁**假阴性**；
- 本项目大量注释/字符串含中文，**这是长期潜伏的雷**。本次因我首次引入非 ASCII 锚点而被暴露。

**修复**：改为收集原始字节、结尾 `String::from_utf8_lossy` 整体还原（仅删除 ASCII 注释字节的子序列仍是合法 UTF-8，故无损）。**ASCII 行为与修复前完全一致**。

**新增回归测试** `strip_preserves_non_ascii_utf8`（单测模块 12/12 全绿）。

---

## 四、修复 3 · wiring spec 锚点同步（2 条）

| 能力 | 原锚点 | 处置 |
|---|---|---|
| `self-verify-in-prompt` | `["cargo test", "COMPILER ERRORS"]` | `COMPILER ERRORS` 是 B 臂机关块字符串，随 **D-8 手术删除**（全仓 0 处代码命中）。**能力本体仍在**：改锚 `loop.rs:2334` 现役原文 `["cargo test", "never claim what you did not verify"]` |
| `constitution-reads-file` | 第 2 环锚 `loop.rs` 的 `constitution::constitution_prompt()` | 该调用点随 **hearth-slim S3（c343031）有意移除**（宪法全文出注入层，3.5K→0，真机实证为 prompt 肥胖元凶；模块本体保留）。改锚 `constitution.rs:35` 的函数签名，**claim 据实收窄**（不再声称"仍注入 build_messages"——那已不成立） |

> 两条均**只改锚点、不改能力语义**；claim 同步收窄，**不留下过度声称**。

---

## 五、修复 4 · xray hash 锁欠账（战报原归类有误）

原战报记载该测试因"**读工作区既存删除项** `docs/xray/wiring-v13.toml`"而红。**核对后不成立**：

- 该文件**存在**，`load_spec` 成功；
- 真因 = **哈希锁未随 `26d760e` 更新**。该提交（`fix(C-1): xray spec 规格同步`）改了 spec 内容，
  提交信息自述"**待编译验证**"——**门禁自此常红 19 天，锁一直没跟上**。

**处置**：独立复算 FNV-1a 64 并与测试报出的实际值交叉验证（两者一致：`0x6791ad6a…`），确认改动正当
（线C D-9 删除 `consecutive_errors>=3` 门控，锚点随现状更新，提交可追溯）后更新锁。
能力条数断言（`==16`）仍独立把关，**防删条**。

> 修锁后，门禁不再停在"哈希不匹配"这一层，而是**暴露出更深层问题**（见第七节）——**信号质量提升**。

---

## 六、修复 5 · 平台门控（4 项，非"修绿"）

以下测试**依赖 POSIX 专有物**，Windows 上必然不可用，**门控而非伪造绿**：

| 测试 | 依赖 | 实证 |
|---|---|---|
| `bash::test_bash_echo` | `echo` | Windows 的 `bash` 解析到 **WSL 存根**，本机**未安装分发** → 返回 `exit code: 1`（实测 stdout 为 UTF-16 中文"没有已安装的分发"） |
| `bash::test_bash_node_check_real_exit_code` | `node --check` + `/tmp` + Linux seccomp | 同上（本测试本就以 Linux seccomp 为验证对象） |
| `bash::test_bash_tail_normal_output` | `seq`/`tail` | 同上 |
| `bash::test_bash_timeout` | `sleep` | 同上；另**原战报已登记该项"含负载抖动"**（实测：同一命令两轮一红一绿），门控同时消除非确定性 |
| `sandbox::test_noop_sandbox_echo` | `echo` 可执行文件 | 同上 |

- 处置：加 `#[cfg(unix)]` + 注释说明依据；**Linux 真机与 CI 覆盖不受影响**。
- **纪律声明**：这是"显式平台门控"，**不是**把环境问题伪装成代码问题；每处均写明实测依据。

---

## 七、⚠ 未修项 · 待顶层裁决（不自主处置）

### 7.1 3 条能力锁定 vs 已被手术删除的能力【冲突】

修好 hash 锁后，`wiring::check` 报出 **3 条能力锁定断裂**：

| 能力 | 声称的保证 | 核验结论 |
|---|---|---|
| `all-done-requires-write` | Done 必须有过真实写盘，否则强制 replan（根治"只读即完成"） | **能力已不存在**：`write_attempted` 现仅存于注释；`loop.rs:2879` 注释明载"all_done 门已随 TaskGraph 删除——完成判定收敛到 run() Done 相位的确定性核验，判定权归模型、护栏只剩预算与核验" |
| `sub-budget-not-halved` | 子代理预算 = `(parent-2).max(8)` | **能力已不存在**：该公式全仓 0 命中；委派路径随 D-4 删除，`spawn_sub_agent` 已成**死代码**（无任何调用点） |
| `no-toolcalls-requires-write` | 纯文本收尾无写 → 必须 replan | **能力已不存在**：分支已坍缩为直接 Done（`loop.rs:3311-3314`）；注释载"框架不再 forced-replan、不再 write-required 逼写" |

**判定**：这三条**不是回归，是 R9 线C手术（D-4/D-8）的有意删除**，spec 未同步。

**关键**：《线C手术·执行窗验收报告》第 119/133/151 行**当年就登记过这个冲突**
（"xray +1（D-8 卡内删除×锁定断言冲突，**已呈顶层裁**）"）——即**已知、已上报、未销账**。

**我为什么停手**：这不是"文档同步"，而是**改质量契约**——
`all-done-requires-write` 锁的是 Hearth 的核心卖点之一"**可信交付/不假完成**"。
删除它 = 从门禁层面正式承认"Done 不再要求真实写盘"。
这属于用户规则里的"**改行为语义 / 动用户可见契约**"，**明文要求停下上报**，故不自主处置。

**三条合法出路（待裁决）**：
1. **认账**：从 spec 删除这 3 条（承认保证已退役）——门禁转绿，但正式放弃 3 项锁定；
2. **收窄**：仅保留仍成立的残留防线（如 `all-done` 可改锚 RC52 改判路径的 `no artifact written this run`），**claim 必须写明作用域已收窄**；
3. **恢复**：在 A 臂重新引入等价的"Done 需真实写盘"护栏——**等于回退已批准的手术**，代价最高。

---

## 八、新发现（已立卡，本卡不擅改）

### D-12 · xray 剥离器**不跟踪字符串状态** → 整文件被误吞，锚点假阴性

**实证（忠实复刻该算法跑 `bash.rs`）**：

```
剥离后长度 = 9081 / 原文 = 43132      ← 约 79% 被吞
剥离后结束状态: inBlock=True           ← 收尾仍停在"块注释"态
```

**机制**：扫描器只看 `/`+`*` 两个字节，**不区分是否位于字符串字面量内**。代码里字符串中的 `/*`
被误判为块注释开头，从此**吞掉后面的一切**。

**后果**：**该文件上挂任何锚点都会假报"缺失"**。`loop.rs` 经同样复刻体检**状态干净**（`inBlock=False`），
故第七节对 `loop.rs` 的 3 条判定**可信**；而 `bash.rs` **在修复前不可用于设锚**（我已据此撤回了为该文件新增的环，并在 spec 中申报原因）。

**未修理由**：需要轻量 Rust 词法（字符串/原始字符串/字符字面量/转义），**独立性强、风险独立**，不应夹带在本卡内。

### D-13 · 仓库提交态非 fmt-clean

`cargo fmt --all` 改动了**我完全未触碰**的 `crates/codex-cli/src/session_store.rs`（6 行）
——说明提交进去的代码本身不满足 `fmt --check`，CI 门禁第 1 道应已红。已立卡。

---

## 九、门禁与红线自查

- 全量 `cargo clippy --workspace --all-targets`：**exit 0**（仅有 1 条既有 `unused variable: r_err`，非本批引入）。
- `cargo fmt --all`：已跑（顺带暴露 D-13）。
- 全量 `cargo test --workspace --no-fail-fast`（实测）：

  | | 修复前 | 修复后 |
  |---|---|---|
  | **失败 target** | **3** | **1** |
  | **失败用例** | **13** | **1** |
  | 全绿 target | — | **60** |

  - `tools-builtin`：**全绿**（原 11 红）
  - `sandbox`：**全绿**（原 1 红）
  - `agent-core` / `codex-cli` / `llm-*` / `service` / `planner` / `agent-types` 等：**全绿**
  - 唯一红点 = `project-xray::real_workspace_wiring_all_green`，**唯一失败用例**，
    原因即第七节的 3 条能力锁定冲突（**待顶层裁决，非缺陷**）。
  - **零新增失败**。

**红线自查**：
- ✅ 未触碰 `crates/sandbox` 的隔离**语义**（仅给一个 POSIX 测试加平台门控，不改隔离行为）；
- ✅ 未动 `a_arm_act_tally` 9 处原位；
- ✅ 未动 `llm-gateway/src/fallback.rs`（S9 de-scope 保持原样）；
- ✅ 未复活 S5 已拆机制；
- ✅ 未做任何 git 写操作以外的仓库操作；**未 push**；
- ✅ 未删除任何被锁定能力（第七节 3 条**原样保留**，等裁决）；
- ✅ 未验证/使用泄露的密钥。

---

## 十、申报（不静默）

1. **未 push**：本批全部为本地 commit。远端 `origin/main` 仍停在 `6871f7a`（未变更）。是否推送待用户指示。
2. **D-11 待裁决**：3 条能力锁定冲突，见第七节——**本卡唯一未闭环项**，且属契约变更，不自主处置。
3. **D-12 未修**：`bash.rs` 等文件在修复前不可设锚，已在 spec 中写明原因，避免制造假红。
4. **D-13/D-5/D-6** 待授权处置（后两项为安全事故事件关联项）。
5. **口径提示**：本批无任何性能/成本数字产出；后续若产出，**一律标 "3.0"**（模型层已切 `agnes-3.0-flash`，与 2.5 时代数据不可比）。
