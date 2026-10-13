//! D-202（2026-10-08, traecode）门禁：`ToolRegistry::load_from_dir` 的**失败不得静默吞掉**。
//!
//! **盲区（自报）**：源码级断言（逐行剥 `//` 注释后）——只钉"失败分支留痕/不再用静默 `if let Ok`"，
//! **不**验证 warn 的运行时输出（无 subscriber 可观测；口径「宁可漏报不误报」）。
//!
//! 病灶：`load_from_dir`（service 启动期 `TOOLS_DIR` 自动发现，`service/src/main.rs`）此前对
//! "目录读不了 / manifest 读不了 / TOML 解析失败 / install 失败"**全部静默吞掉**，函数仍返回
//! `Ok(count)` ⇒ 操作者放进 `TOOLS_DIR` 的工具（尤其 TOML 语法/字段错）被**无声丢弃**，而日志
//! 只在 `discovered > 0` 时打印，坏 manifest 现场无任何提示。
//! 修复＝每个失败分支 `tracing::warn!` 留痕（容错跳过语义保留），`read_dir` 失败显式 `Err`。

use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn manifest_load_does_not_silently_drop() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/registry.rs");
    let src = std::fs::read_to_string(&p).expect("crates/tool-runtime/src/registry.rs 必须存在");
    let code = code_only(&src);

    assert!(
        !code.contains("if let Ok(entries) = std::fs::read_dir"),
        "D-202：`load_from_dir` 不得再用 `if let Ok(entries) = std::fs::read_dir(..)` —— 目录读取失败会被静默吞成 `Ok(0)`"
    );
    assert!(
        code.contains("tracing::warn!"),
        "D-202：`load_from_dir` 的失败分支必须 `tracing::warn!` 留痕（manifest 读/解析失败此前静默丢弃）"
    );
}

/// 行为 pin（非 RED）：**容错语义必须保留**——一个坏 manifest 不得让**其余好工具**也加载不了。
#[test]
fn manifest_load_tolerates_broken_entry() {
    let dir = std::env::temp_dir().join(format!("toolreg-{}", std::process::id()));
    let good = dir.join("good");
    let bad = dir.join("bad");
    std::fs::create_dir_all(&good).unwrap();
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(
        good.join("manifest.toml"),
        "name = \"t-good\"\ndescription = \"d\"\nversion = \"1\"\nsha256 = \"\"\ncommand = \"\"\ntags = []\n",
    )
    .unwrap();
    // 语法错误：不得解析
    std::fs::write(
        bad.join("manifest.toml"),
        "name = \"oops\"\nthis is not toml =\n",
    )
    .unwrap();

    let reg = tool_runtime::ToolRegistry::new();
    let n = reg.load_from_dir(&dir).expect("目录可读时必须 Ok");
    assert_eq!(n, 1, "只应加载 1 个（坏 manifest 容错跳过，好工具仍加载）");
    let _ = std::fs::remove_dir_all(&dir);
}
