//! codex-xray CLI — CI 第四门（v13 S1/S2）。
//!
//! 用法:
//!   codex-xray scan   [--root .] [--out xray-out/facts.json]
//!   codex-xray wiring [--root .] [--spec docs/xray/wiring-v13.toml]
//!
//! wiring: 任一 severity=red 的能力链断裂 → exit 1（使 CI / VM gate 失败）。

use clap::{Parser, Subcommand};
use project_xray::{facts, wiring};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "codex-xray",
    about = "project X-ray: facts scan + wiring assertions (the 4th CI gate)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scan the workspace and write facts.json (real metrics, no hand counting).
    Scan {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "xray-out/facts.json")]
        out: PathBuf,
    },
    /// Run wiring assertions; a red break exits non-zero.
    Wiring {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "docs/xray/wiring-v13.toml")]
        spec: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Scan { root, out } => {
            let f = facts::scan(&root)?;
            facts::write_facts(&f, &out)?;
            println!(
                "xray scan: {} members, {} rs files, {} LOC, {} test declarations -> {}",
                f.workspace_members,
                f.rs_files,
                f.total_loc,
                f.test_declarations,
                out.display()
            );
            Ok(())
        }
        Cmd::Wiring { root, spec } => {
            let spec = wiring::load_spec(&spec)?;
            let results = wiring::check(&root, &spec);

            let mut broken_red = 0usize;
            for cap in &results {
                let status = if cap.broken { "BREAK" } else { "PASS " };
                println!("[{status}] {} ({}) — {}", cap.id, cap.severity, cap.claim);
                for link in &cap.links {
                    if !link.ok {
                        println!("    x {} — {} [{}]", link.file, link.detail, link.meaning);
                    }
                }
                if cap.broken && cap.severity == "red" {
                    broken_red += 1;
                }
            }

            let total = results.len();
            let broken = results.iter().filter(|r| r.broken).count();
            println!(
                "xray wiring: {}/{} pass, {broken} broken ({broken_red} red)",
                total - broken,
                total
            );

            if wiring::has_red_break(&results) {
                eprintln!("xray wiring: RED BREAK — a locked capability regressed. Gate failed.");
                std::process::exit(1);
            }
            Ok(())
        }
    }
}
