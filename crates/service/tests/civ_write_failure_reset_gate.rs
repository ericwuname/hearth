//! D-182（P1-145）：**civ 写失败计数必须是"连续"语义**（成功即清零）——源码级门禁。
//!
//! 背景（病灶，HA/可用性）：
//!   · `civ_write_failures`（`main.rs` `CivWriterAdapter`）**只增不减**——两处 `fetch_add(1)`，
//!     全仓**无任何 `store(0)`/清零**。
//!   · `routes::readyz` 在 `>5` 时判 **503**（`civ_store_degraded`）。
//!   · 但 `CivWriterAdapter` 的注释自称"**连续**失败"——**实现是累计失败**（声称≠实现）。
//!   ⇒ 后果：一次**瞬时**抖动（如某租户档位目录短暂不可写）累计 >5 次后，该实例
//!     **永久**不 ready（负载均衡永久摘除，直到进程重启）——即便故障早已恢复。
//!     永不恢复的 readiness 探测把"可自愈的抖动"放大成"实例下线"。
//!
//! 修复口径：抽 `routes::record_civ_write(&AtomicU64, ok)` —— `ok=true` **清零**、
//! `ok=false` 自增（真·连续语义）；`CivWriterAdapter` 的成功路径清零、两条失败路径自增。
//!
//! 本门禁是**源码级**（不启动 service，故修复前后都能编译）：读 `src/main.rs`、逐行剥 `//`
//! 注释后断言 —— ① 存在成功清零路径 `record_civ_write(&self.failures, true)`；
//! ② 存在失败自增路径 `record_civ_write(&self.failures, false)`；③ **不再**直连 `fetch_add`
//! （防回潮到"只增不减"）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 不端到端跑 agent 触发真实 civ 写失败（需真实 LLM 跑通一轮）；计数语义由
//!     `routes` 的 `record_civ_write` 单测覆盖，本门禁只 pin **接线**。
//!   - 剥注释用行级 `//`（与既有源码级门禁同口径）。

/// 读源文件并逐行剥掉 `//` 之后的注释（保留代码）。
fn code_only(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn civ_write_failure_counter_resets_on_success() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs");
    let src = std::fs::read_to_string(path).expect("read main.rs");
    let code = code_only(&src);

    assert!(
        code.contains("record_civ_write(&self.failures, true)"),
        "D-182：`CivWriterAdapter` 必须在**写成功**时调 `record_civ_write(&self.failures, true)`\
         ——否则失败计数只增不减，一阵瞬时抖动会让 `/readyz` **永久** 503（HA：实例被永久摘除）"
    );
    assert!(
        code.contains("record_civ_write(&self.failures, false)"),
        "D-182：失败路径必须调 `record_civ_write(&self.failures, false)`（自增）"
    );
    assert!(
        !code.contains("failures") || !code.contains(".fetch_add("),
        "D-182 回潮：`CivWriterAdapter` 又直连 `fetch_add` 了——那等于回到'只增不减'，\
         请改用 `routes::record_civ_write`（成功清零 / 失败自增）"
    );
}
