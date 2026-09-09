//! codex — 旧名别名（docs/hearth-naming.md：deprecated 别名，保留不删）。
//! 与 hearth 完全同逻辑。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hearth::hearth_main().await
}
