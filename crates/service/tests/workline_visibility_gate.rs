//! 门禁：**组合根（`main.rs`）不得再构造全局 work line store**（D-138，2026-10-04, traecode）。
//!
//! 背景：work line 的**可见面**是 per-user store（`per_user.workline_for(uid)`，落在
//! `MEMORY_DIR/<uid>/workline.jsonl`），HTTP `/api/v1/workline*` 全部读它。而 `main.rs`
//! 曾**额外**构造一个全局 store（`MEMORY_DIR/workline.jsonl`）并挂一个 "v10.4 60s 后台
//! 调度器"，对**所有** pending 节点每 60s `progress + 1.0`（status 传 `None` ⇒ 状态
//! 永不流转、进度无界增长，且每次 `update` 重写整份 jsonl）。该全局 store **没有任何
//! 读取方**（`AppState` 早于 D-108 已删除该字段；唯一使用者是调度器自己）⇒ "只写不读"
//! 死物，且"无人做事却每 60s 涨进度"本身是**谎报进度**。与 D-108/D-109 退役全局文明线同源。
//!
//! 判据（源码级，宁可漏报不误报）：逐行剥掉 `//` 行注释后的 `main.rs` **全文**不得出现
//! `WorkLineStore`。理由：组合根不应自行 `new` 任何 work line store——可见面由
//! `per_user.rs` 按用户惰性构造（那里同样是 "workline.jsonl"，但落在 `<uid>/` 下）。
//! 若确需在组合根构造全局档，请连同读侧一起设计并同步本门禁。
//!
//! 文件头自报盲区：① 只扫 `crates/service/src/main.rs` 全文，不追 per-user 归属链
//! （那由 `per_user.rs` 单测覆盖）；② 只钉"组合根是否构造全局 store"，不钉调度/进度
//! 语义（那属行为层，见 D-138 说明）。

use std::path::PathBuf;

#[test]
fn composition_root_does_not_build_global_workline_store() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("main.rs");
    let raw = std::fs::read_to_string(&p).expect("crates/service/src/main.rs 必须存在");
    // 剥掉 `//` 行注释：退役说明注释可照常点名 `WorkLineStore`，而真正的**代码**一旦
    // 构造全局 store（如 `WorkLineStore::new(...)`）即报红。
    let src: String = raw
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !src.contains("WorkLineStore"),
        "`main.rs`（组合根）不得再构造**全局** work line store —— 全仓无人读它，写进去的\
         进度永久不可见，且会每 60s 无界增长并重写整份 jsonl（D-138 病灶）。work line 的\
         可见面由 `per_user.workline_for(uid)` 提供。若确需在组合根构造全局档，请连同读侧\
         一起设计，并同步更新本门禁。"
    );
}
