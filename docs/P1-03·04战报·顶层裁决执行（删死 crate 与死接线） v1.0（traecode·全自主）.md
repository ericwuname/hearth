# P1-03 / P1-04 战报 · 顶层裁决执行：删除死 crate 与死接线 v1.0

- **出品**：traecode　**日期**：2026-10-01　**授权**：用户全权 + **本轮七项裁决**
- **基线**：`p0-usability-01` @ `89b60f0`
- **范围**：执行裁决 3（`project-sync` 删除）与裁决 4（`retriever` / `lsp_bridge` 死接线删除）
- **结论先行**：两处均按裁决删除；**门禁目标数 62 → 60**；**零行为回归**。

---

## 一、顶层七项裁决（登记）

| # | 事项 | 裁决 | 处置 |
|---|---|---|---|
| 1 | bash 出网无治理（D-35） | **接受**（不治理） | 登记为**显式风险接受**，D-35 close |
| 2 | 3 个文件里有真 key（D-37） | **不清理** | 登记为**显式风险接受**，D-37 close |
| 3 | `project-sync` 死 crate | **删除** | **本卡 P1-03** |
| 4 | `retriever` / `lsp_bridge` 死接线 | **删除** | **本卡 P1-04** |
| 5 | 2 条退役能力（yellow） | **追认** | 待办，单独立卡 |
| 6 | 有界读入辅助三处重复（D-33） | **收敛** | 待办，单独立卡 |
| 7 | Linux 交叉类型检查（D-30） | **列入常规门禁** | 待办，单独立卡 |

> 裁决 1/2 是**风险接受**，不是"问题不存在"：D-35（bash 可 `curl` 直连任意地址）与
> D-37（3 处真 key 留在公开树里）依然成立，只是**不再作为待修项**。
> 二者都写进债队列终态，供日后追溯。

---

## 二、P1-03 · 删除 `project-sync`（整 crate）

**依据**：P1-01 体检实证——**无任何 crate 依赖它**，仅挂在工作区 `members` 里。

**动作**：
`git rm -r crates/project-sync`（12 文件 / ~90 KB）+ 从 `members` 移除 +
订正 [atomic.rs](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/crates/tools-builtin/src/atomic.rs)
文档注释里对这已删文件的引用（否则注释会指向不存在的路径）。

---

## 三、P1-04 · 删除 `retriever` / `lsp_bridge` 死接线

### 3.1 这不是"删两个字段"，而是"停掉一笔真实启动成本"

P1-01 体检只说"字段只写不读"。执行删除时把整条链读完，看清了它的**实际代价**——
service 启动时会：

1. **扫描 `CODE_DIR` 下所有 `.rs` 文件、逐个读进内存、建 Tantivy 索引**
   （`main.rs` 原 511-538 行）；
2. 视 `LSP_ENABLED=1` **起一个 rust-analyzer 桥**；
3. 然后 `sessions.set_retriever(..)` / `set_lsp_bridge(..)` ——
   注入一个 **agent-core 从不读取** 的字段。

即：**启动成本是真的，收益是零**。

### 3.2 删除清单（三层贯通）

| 层 | 删除内容 |
|---|---|
| `agent-core` | 两个字段（`lsp_bridge: Arc<dyn LspBridge>` / `retriever: Option<Arc<dyn Retriever>>`）+ 两个 setter + 构造初始化 + 两个 `use` 里的 trait 名（`Diagnostic` / `SearchResult` **保留**，`Event` 变体仍在用） |
| `agent-runtime` | 两个字段 + 两个 setter + 构造初始化 + **注入块** + 两个 `use`；`Cargo.toml` 去掉 `retriever` / `lsp-bridge` 依赖 |
| `service` | 整段 retriever 启动构建 + 整段 LSP 接线 + 两个 `use`；`Cargo.toml` 去掉 `retriever` / `lsp-bridge` / **`code-index`**（该块是它唯一使用者） |

### 3.3 退役 `known_dead_wiring_marker` 测试

该测试是**为"等裁决"而生**的：它断言两个字段仍存在且"赋值数 == 总出现数"。
裁决为删除后，`total >= 1` 已不可能成立 → **退役**，并在原位置留注释说明
"为何退役而不是改写"（改写成一个恒真的空壳只会掩盖"这条线已经没人守了"）。

### 3.4 未夹带：登记 D-40

`Event::LspDiagnostics` / `Event::Retrieval` 两个事件变体**仍在**，且全仓
**无生产者**（`loop.rs:2625` 还有一段消费它们的提示词格式化逻辑）。
删除事件变体会动到事件流的外部形状 → **另立卡**，不在本卡内夹带。

---

## 四、门禁

| 项 | 结果 |
|---|---|
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo fmt --all -- --check` | 干净 |
| `cargo clippy --workspace --all-targets` | **exit 0** |
| `cargo test --workspace --no-fail-fast` | **exit 0**；**60 target 全 ok**（62 → 60，`project-sync` 贡献 2）；`FAILED` **0** |

**红线自查**：未改沙箱隔离语义；未动 `a_arm_act_tally` / `fallback.rs`；未复活 S5；
无破坏性 git 操作；未使用/未验证任何泄露密钥。
**行为变更声明**：service 启动时不再建检索索引、不再起 rust-analyzer；
`EMBED_MODEL` / `CODE_DIR` / `LSP_ENABLED` 三个 env 随之失效——
因这两条链**从未被任何消费点读取**，故**无用户可见行为回归**。

---

## 五、方法论沉淀（第 13 个变体）

> **"死代码不是零成本——它把成本花在了别处。"**

`retriever` 字段本身不花一分钱；但它**牵引出**了"启动时全量扫盘建索引 +
可能起 rust-analyzer"这条真实支线。**"只写不读"的判据只看到了字段，
看不到它背后的启动成本**。

**固化为检查项**：
1. **删除"死接线"时，必须顺着赋值点向上游走到源**（谁在调 setter？它为此付了什么代价？），
   再决定删到哪一层——本卡若只删字段，会留下"空建索引"的更隐蔽浪费；
2. **凡"为等裁决而写的钉住测试"，裁决落地时一并退役**，
   并在原位写明"为何退役"，避免后人误以为该约束仍有人守。
