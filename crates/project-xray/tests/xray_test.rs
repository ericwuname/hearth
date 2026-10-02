//! 集成测试：对真实 workspace 跑 scan + wiring。
//!
//! 关键：`real_workspace_wiring_all_green` 让 wiring 断言也被普通
//! `cargo test` 覆盖——即使某个环境忘了单独跑 codex-xray，
//! 三门里的 test 门也会抓住接线回归（双保险）。

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn real_workspace_scan_sane() {
    let facts = project_xray::facts::scan(&workspace_root()).expect("scan workspace");
    assert!(
        facts.workspace_members >= 20,
        "expected >=20 crates, got {}",
        facts.workspace_members
    );
    assert!(facts.rs_files >= 50, "rs files: {}", facts.rs_files);
    assert!(facts.total_loc >= 10_000, "LOC: {}", facts.total_loc);
    assert!(
        facts.test_declarations >= 100,
        "test declarations: {}",
        facts.test_declarations
    );
}

#[test]
fn real_workspace_wiring_all_green() {
    let root = workspace_root();
    let spec_path = root.join("docs/xray/wiring-v13.toml");
    let spec = project_xray::wiring::load_spec(&spec_path).expect("load wiring-v13.toml");
    assert!(
        // P0-4 (audit-fix): 阈值从 >=7 收紧为精确 15（删任意 1 条即红）。
        // v0.2.2 天赋: +talent-style-injected → 16。
        spec.capabilities.len() == 16,
        "wiring requires exactly 16 capabilities (防删 1 条仍绿), got {}",
        spec.capabilities.len()
    );
    // P0-4: 锁 spec 哈希——防「改断言让门禁变绿」。
    let spec_bytes = std::fs::read(&spec_path).expect("read wiring-v13.toml");
    // P2-issue-1 (audit-fix): 改用 FNV-1a 64-bit 稳定哈希，替代 DefaultHasher。
    // DefaultHasher 的算法标准库未规定、随 rustc 版本变化，导致同一份 spec 在
    // Docker(1.82)/VM(1.97.1) 等环境必红、只在作者本机 rustc 恰等于锁值时绿。
    // FNV-1a 算法公开确定，跨 rustc/平台一致。期望值由当前 wiring-v13.toml 计算。
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in &spec_bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let spec_hash = h;
    assert!(
        // 2026-09-30 更新锁：原锁 0x0444c6bfbbbb0ea2（2026-08-25, v0.2.2 天赋）。
        // 变更来源 = commit 26d760e「fix(C-1): xray spec 规格同步」——该提交把
        // `experience-adaptive-switch` 的锚点从 `consecutive_errors >= 3` 更新为
        // `experience_store`/`set_experience_store`，理由：线C手术 D-9 已删除该门控
        // （B 臂 do_reflect 随相位机拆除，D-7 起恒 0 死分支），机制本体保留 → A 臂行为零变化。
        // **该改动是有意的、可追溯的**，故按本断言自述的约定（"若是有意改动 spec，请更新此锁"）
        // 更新哈希；能力条数断言（==16）仍独立把关，防删条。
        // 注：26d760e 提交信息自述"待编译验证"，本锁因此长期未更新 → 门禁常红 19 天。
        // 2026-10-01 复算（顶层裁决5「追认」）：把 sub-budget-not-halved /
        // no-toolcalls-requires-write 两条的「待顶层追认」文案改为「顶层已追认」。
        // **纯文案**——能力条数（仍是 16）、severity（仍是 yellow）、各链锚点均零变化。
        // 2026-10-01 复算（D-48 重接线）：`civ-auto-written` 的 claim 与链锚点改为
        // 锚定**真实唯一生产接线点**（run() 收尾 `note_civ_outcome`），修掉"锚点命中
        // 赋值语句"造成的假绿。**纯锚点/文案**——能力条数（仍 16）、severity（仍 red）不变。
        // 2026-10-01 复算（D-75，P1-38）：`constitution-reads-file` 第 1 环锚点随
        // 「constitution.md 改有界读入」同步——`read_to_string` → `read_file_text_capped_std`
        // （能力不变，实现换底；锚点若不改，本门禁会如实报红，见 P1-38 先红过程）。
        // 2026-10-01 复算（D-83）：`subagent-uses-readonly` 条目退役——子代理委派子系统
        // 整段删除后，其 `read_only_view()` 唯一生产调用点消失，severity 由 red 降为 yellow
        // （不阻断门禁）。**纯退役登记 + 条数不变（仍 16）**。
        // 2026-10-02 复算（D-107）：`experience-prune-wired` 的 claim 与锚点收窄——
        // 原第二环锚 `observer_experience.upgrade_core()` 因**依赖无写入方的
        // `reference_count`（恒空查询）**随 D-100 裁决（联网核实 Reflexion/OEP
        // + 本仓 v17/v18 实测）退役；经验库定位为**只写审计档**。剪枝（prune）
        // 仍每小时真实执行 ⇒ 该项收窄为真实生效的那一环。
        // **纯锚点/文案**——能力条数（仍 16）、severity（仍 red）均不变。
        // 2026-10-02 复算（D-109）：`civ-auto-written` 的后两环锚点由
        // `set_civ_writer(`（单个全局 writer，写全局档 ⇒ API 不可见）改为
        // `set_civ_writer_factory(`（按归属用户构造 writer，落到 `per_user.civ_for(uid)`
        // 的可见档）。**纯锚点/文案**——能力条数（仍 16）、severity（仍 red）不变。
        spec_hash == 0x457ac49206c92a81, // 2026-10-02 复算（FNV-1a 64；D-109 civ 写入归属链）
        "wiring spec hash changed — actual=0x{spec_hash:016x}；若是有意改动 spec，请更新此锁",
    );

    let results = project_xray::wiring::check(&root, &spec);

    // 2026-09-30 口径对齐（traecode）：本测试原先无条件要求 `broken.is_empty()`，
    // 与引擎实际语义**不一致**——引擎的 `has_red_break()` 只把 `severity=red` 的断裂
    // 判为门禁失败（CI 第 4 道门 `codex-xray wiring` 即用该函数）。
    // 已退役能力被登记为 `yellow` 留痕后，旧断言会让本测试单独红 → 与 CI 门互相矛盾。
    // 现改为与引擎同口径：**只有 red 断裂才失败**；yellow 断裂**打印出来**（不静默留痕）。
    let fmt_break = |r: &project_xray::wiring::CapabilityResult| -> String {
        let details: Vec<String> = r
            .links
            .iter()
            .filter(|l| !l.ok)
            .map(|l| format!("{}: {}", l.file, l.detail))
            .collect();
        format!("{} [{}]", r.id, details.join("; "))
    };

    let yellow: Vec<String> = results
        .iter()
        .filter(|r| r.broken && r.severity != "red")
        .map(fmt_break)
        .collect();
    if !yellow.is_empty() {
        eprintln!(
            "[wiring] {} 条**非阻断**断裂（非 red，已登记退役·顶层已追认 2026-10-01）——列此留痕，不阻断门禁：{yellow:#?}",
            yellow.len()
        );
    }

    let red: Vec<String> = results
        .iter()
        .filter(|r| r.broken && r.severity == "red")
        .map(fmt_break)
        .collect();
    assert!(
        red.is_empty(),
        "wiring assertions broken (red): {red:#?} — a locked capability regressed!"
    );
}
