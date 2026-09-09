//! hearth — 主二进制入口（薄壳，逻辑全在 lib）。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hearth::hearth_main().await
}
