# P0-05 战报 · 服务端 panic / 锁中毒 / 在途计数泄漏的 DoS 面 v1.0

- **出品**：traecode　**日期**：2026-10-01　**授权**：用户全权（"全部自主，无需汇报"）
- **基线**：`p0-usability-01` @ `539f4fc`
- **范围**：D-24（`per_user.rs` 请求路径 `.expect()` + 锁中毒）+ D-25（`routes.rs` 限流中间件 `lock().unwrap()`）+ **同批扫出的同类项**
- **结论先行**：**修 4 处**（civ/workline 存储创建、限流中间件、webhook 注册表、用户密钥表），
  其中 **1 处为本卡新发现的真缺陷**（限流在途计数在 handler panic 时**单调泄漏** → 全局 429）。

---

## 一、缺陷族：一句话

> **一次发生在"错误的地方"的 panic，被放大成"此后永远"的失败。**

三个放大机制，本卡全部切断：

| 放大机制 | 机制说明 | 放大倍数 |
|---|---|---|
| **锁中毒（poison）** | `std::sync::*` 锁在某线程 panic 后被永久标记中毒，此后每次 `lock().unwrap()` 都 panic | **1 次 panic → 此后每次访问都失败** |
| **panic 发生在请求路径** | `per_user` 的存储创建失败率与磁盘状态挂钩，属**可达**错误，却被写成 `expect` | 1 次 I/O 错误 → 1 次连接被撕 |
| **计数器在 unwind 中泄漏** | 尾部手动 `fetch_sub` 在 panic 展开时**不执行** | 累积到上限 → **永久 429** |

三者**可串联**：磁盘满 → `civ_for` `.expect()` panic → ① 连接断（计数泄漏）② civ 锁中毒 →
`/api/v1/civilization` 全站永久 500，**且**限流配额被逐步吃掉。

---

## 二、D-24 · `per_user.rs`：可达错误 + 锁中毒双杀

**病灶**（[per_user.rs](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/crates/service/src/per_user.rs) 原实现）：

```rust
let mut map = self.civ.write().unwrap();          // ← panic 时**仍持有**写锁
map.entry(user_id.to_string())
    .or_insert_with(|| {
        Arc::new(CivilizationStore::new(&dir, "civ.jsonl")
            .expect("per-user civ store"))        // ← ①在请求路径上 panic
    })
    .clone()
```

`CivilizationStore::new` 内部 `create_dir_all` + 打开 jsonl —— **磁盘满 / 目录只读 /
`base_dir/user_id` 被同名文件占用**都会失败。这不是"不可能发生"的断言，而是**可达错误**。

**修复**：
1. 签名改为 `anyhow::Result<Arc<…>>`，失败 **上抛**（handler 转 500，只影响这一次请求）；
2. 锁访问走 `recover`（中毒不升级）。

> 注：`workline_for` 是**同源同修**——同一 `.expect()` 模式、同一把写锁的中毒链。

**回归测试**（`per_user::tests`）：
- `test_p0_05_civ_for_failure_is_error_not_panic`：把 `base_dir` 指向一个**普通文件**
  （跨平台可控地让 `create_dir_all` 必失败），断言返回 `Err`；**并二次调用**证明锁未被中毒。
  修复前该断言处**直接 panic**。
- `test_p0_05_per_user_store_happy_path_caches`：正常路径行为不变（同 user 复用同一实例）。

---

## 三、D-25 + 新发现 · `routes.rs` 限流中间件

### 3.1 子项 A：`lock().unwrap()`（D-25，已修）

三处 `.lock().unwrap()` → 换 `recover(...)`。守卫的数据是 `HashMap<IpAddr, usize>`
计数器，没有跨字段不变式，中毒无保留价值。

### 3.2 子项 B：**在途计数在 panic 时单调泄漏**（**本卡新发现**，已修）

原实现在函数**尾部**手动归还：

```rust
let resp = next.run(req).await;        // ← handler panic 在这里 unwind
P1_TOTAL_INFLIGHT.fetch_sub(1, …);     // ← 永不执行
```

**机制**：`next.run(req).await` 展开时，其后的 `fetch_sub` 与 per-IP 回退语句**全部跳过**。

**放大链**（两条独立出口）：
- per-IP：同一 IP 泄漏 50 次 → 该 IP **永久 429**；
- 全局：泄漏 500 次 → **整站永久 429**（`/healthz`/`/readyz` 虽豁免，业务面已全灭）。

**触发面**：**任何** handler panic 都行 —— 包括本卡修复前的 `civ_for` `.expect()`。
即"一次磁盘写满"可以同时换来"civ 接口永久 500"**和**"整站配额被吃掉"。

**修复**：改为 **RAII 守卫** `InflightGuard`——名额由守卫持有，**`Drop` 在 unwind 时仍会执行**，
故计数成对归还。名额获取/上限判定抽为 `enter()`（含全局拒绝时回退 per-IP 的原有语义）。
**未改**任何限额数值（`P1_MAX_PER_IP = 50` / `P1_MAX_TOTAL = 500`）与 429 语义。

**回归测试**（`routes::p0_05_tests`）：

```rust
let r = catch_unwind(AssertUnwindSafe(|| {
    let _g = enter(Some(ip)).expect("容量内应放行");
    panic!("simulated handler panic");
}));
// 断言 per-IP 与全局计数**均已归还**
```

另含：正常路径占用/归还、per-IP 条目清理（防 map 无界增长）、per-IP 上限仍拒绝。
三个断言写在**同一个**测试函数内——全局静态计数器并行会互相干扰。

---

## 四、同类项 · 一并清理（webhook / user）

| 位置 | 影响面 | 处置 |
|---|---|---|
| `webhook.rs:36,104,111` | 注册/列举 webhook 的请求路径 | `recover` |
| `user.rs:27,32` | **每个请求的鉴权路径**（`find`） | `recover` |

这两处的"持锁 panic"本身不易触发，但它们与 D-24/D-25 **是同一缺陷类**，
留着等于明知故留一条全站级失败通道；修复是零风险的一行替换。

---

## 五、新增基础设施 · `service/src/lock.rs`

```rust
pub fn recover<T>(r: LockResult<T>) -> T {
    r.unwrap_or_else(std::sync::PoisonError::into_inner)
}
```

模块注释写清了**为什么这里忽略中毒是安全的**（守卫数据无跨字段不变式）
与**为什么不等于静默吞错**（真正的失败仍由 `Result` 上抛）。
自带单测 `test_p0_05_recover_survives_poisoned_lock`：先制造中毒（持锁 panic），
断言裸 `lock()` 确实 `Err`（**这正是旧 `.unwrap()` 会炸的地方**），再断言 `recover` 取回数据。

---

## 六、门禁

| 项 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | 干净（exit 0） |
| `cargo clippy --workspace --all-targets` | **exit 0** |
| `cargo test --workspace --no-fail-fast` | **exit 0**；**61 target 全 ok**；"failed" 非零计数 **0** |
| `service` 单测 | 9 passed（新增 4 条） |
| `service` 集成测试 | 9 passed |

**红线自查**：未改沙箱隔离语义；未动 `a_arm_act_tally`；未动 `fallback.rs`；未复活 S5；
无破坏性 git 操作；未使用/未验证任何泄露密钥；**未改任何限流阈值或用户可见契约**
（`civ_for` 签名变化属 crate 内部 API，调用点仅 5 处，行为变化只有"panic → 500"）。

---

## 七、方法论沉淀（第 6 个变体的样子）

前四张卡的共性是"**假设某物存在/成立，而它在别处不成立**"。本卡是同一主线的**第 6 个变体**：

> **"假设错误会就地停下。"**

- 旧代码假设：`civ_for` 不会失败 → 用 `expect`；
- 旧代码假设：`next.run` 不会 panic → 在尾部归还计数；
- 旧代码假设：一次 panic 只影响一次请求 → 忽略了**锁中毒**与**计数器泄漏**这两种"跨请求记忆"。

**固化为检查项**（补充至 D-23 专项，未实施）：
1. **凡 `unwrap`/`expect` 落在请求路径（或任何长驻服务的循环里），一律视为缺陷** ——
   错误必须上抛或降级，**不得**靠"它不会失败"过关；
2. **凡"获取资源 → 使用 → 归还"跨越 `.await` 或可能 panic 的调用，一律用 RAII 守卫**，
   绝不在尾部手动归还；
3. **凡 `std::sync` 锁的守卫数据没有跨字段不变式，一律用 `recover` 而非 `.unwrap()`**。
