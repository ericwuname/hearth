//! 门禁：**Hearth 用户配置（config.toml）的路径解析必须只有一处定义**（D-144，2026-10-04, traecode）。
//!
//! 背景（真实缺陷，D-144）：`codex-cli` 的 `config_path()` 是 **APPDATA 感知**的
//! （Windows → `%APPDATA%\hearth\config.toml`；其他 → `$XDG_CONFIG_HOME` 或 `$HOME/.config`），
//! 而 `service/src/main.rs`（RC13 起，为"补读 config.toml 的 egress_allowlist"）却**写死**
//! `$HOME/.config/hearth/config.toml`（全平台）。二者在 **Windows 上分叉** ⇒ 用户用
//! `hearth config set` 写进 `%APPDATA%\hearth\config.toml` 的白名单，service 会话**读不到**
//! ——RC13 自称"与 CLI 侧 merge_allowlist 同构 / 代码 ✓"，实为**声称≠实现**（且该条当时
//! 就标注 `NEEDS-RERUN`，从未在真机验证）。同族：D-33「一处定义」口径、D-109 的归属链。
//!
//! 处置（单一真相源）：解析逻辑上收到 `agent_types::hearth_config_path()`，`codex-cli` 与
//! `service` 均**只调用**它。本门禁钉死该收敛——**config.toml 的路径拼接字面量不得出现在
//! `codex-cli/src` 与 `service/src` 里**（允许出现在 `agent-types`）。
//!
//! 判据（宁可漏报不误报）：剥 `//` 注释后，非测试代码里出现以下任一形态即报红：
//!   · `join("hearth").join("config.toml")`
//!   · `".config/hearth/config.toml"`
//!
//! 文件头自报盲区：
//! ① 只扫 `crates/codex-cli/src` 与 `crates/service/src` 两个目录。
//! ② 只做**文本级**逐行匹配；等价的其它拼法（如经中间变量拆分、`format!` 拼串）不在射程内。
//! ③ 不校验 `agent_types::hearth_config_path()` 的**平台矩阵正确性**（那由 agent-types 的
//!    `hearth_config_path_from` 单测负责）。

use std::path::{Path, PathBuf};

/// 仓库根：`crates/service/tests/xxx.rs` → 上溯三级。
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

/// 逐行剥掉 `//` 之后的部分（注释里提到旧写法不算回归）。
fn code_part(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// 该行是否在**就地拼** config.toml 路径（闻起来像"又一处定义"）。
fn is_local_config_path_build(code: &str) -> bool {
    code.contains("join(\"hearth\").join(\"config.toml\")")
        || code.contains("\".config/hearth/config.toml\"")
}

#[test]
fn config_toml_path_has_single_source() {
    let root = workspace_root();
    let scan_dirs = [
        root.join("crates").join("codex-cli").join("src"),
        root.join("crates").join("service").join("src"),
    ];

    let mut files = Vec::new();
    for d in &scan_dirs {
        assert!(d.is_dir(), "缺少源码目录：{}", d.display());
        collect_rs(d, &mut files);
    }
    assert!(
        files.len() >= 5,
        "扫描面过小（{} 个源文件）——门禁自身可能失效",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let mut depth: i32 = 0;
        let mut test_base: Option<i32> = None;
        let mut pending_cfg_test = false;
        for (i, raw) in text.lines().enumerate() {
            let code = code_part(raw);
            let trimmed = code.trim();
            let opens = code.matches('{').count() as i32;
            let closes = code.matches('}').count() as i32;
            if trimmed == "#[cfg(test)]" {
                pending_cfg_test = true;
            }
            let inside_test = test_base.is_some_and(|b| depth > b);
            if !inside_test && is_local_config_path_build(code) {
                offenders.push(format!(
                    "{}:{} {}",
                    f.strip_prefix(&root).unwrap_or(f).display(),
                    i + 1,
                    raw.trim()
                ));
            }
            if pending_cfg_test && trimmed.starts_with("mod ") && opens > 0 {
                test_base = Some(depth);
                pending_cfg_test = false;
            }
            depth += opens - closes;
            if test_base.is_some_and(|b| depth <= b) {
                test_base = None;
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "config.toml 路径被**就地拼**（应只有一处定义）：请统一改走 \
         `agent_types::hearth_config_path()`——CLI 与 service 必须解析到**同一路径**，\
         否则 config 里的 egress_allowlist 等设置会在其中一侧静默不生效（D-144 实证：\
         Windows 上 CLI 用 %APPDATA%、service 用 $HOME/.config，二者分叉）。命中：{offenders:#?}"
    );
}
