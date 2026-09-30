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
        spec_hash == 0x0a5929b757504297, // 2026-09-30 复算（FNV-1a 64；本批同步 2 条锚点后重锁）
        "wiring spec hash changed — actual=0x{spec_hash:016x}；若是有意改动 spec，请更新此锁",
    );

    let results = project_xray::wiring::check(&root, &spec);
    let broken: Vec<String> = results
        .iter()
        .filter(|r| r.broken)
        .map(|r| {
            let details: Vec<String> = r
                .links
                .iter()
                .filter(|l| !l.ok)
                .map(|l| format!("{}: {}", l.file, l.detail))
                .collect();
            format!("{} [{}]", r.id, details.join("; "))
        })
        .collect();
    assert!(
        broken.is_empty(),
        "wiring assertions broken: {broken:#?} — a locked capability regressed!"
    );
}
