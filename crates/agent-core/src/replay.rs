// v7.0: Simple session replay — reads stored events from JSONL file.
use serde::Serialize;
use std::io::{BufRead, BufReader};

/// Minimal replay result — just the event summary.
#[derive(Debug, Serialize)]
pub struct ReplaySummary {
    pub session_id: String,
    pub total_events: usize,
    pub phases: Vec<String>,
    pub errors: Vec<String>,
}

/// Load a session from JSONL file and produce a replay summary.
pub fn replay_session(store_dir: &str, session_id: &str) -> anyhow::Result<ReplaySummary> {
    let path = std::path::Path::new(store_dir).join(format!("{}.jsonl", session_id));
    let f = std::fs::File::open(&path)?;
    let mut phases = Vec::new();
    let mut errors = Vec::new();
    let mut total = 0;
    for line in BufReader::new(f).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
            total += 1;
            if let Some(et) = event["event_type"].as_str() {
                match et {
                    "phase" => {
                        if let Some(p) = event["payload"]["phase"].as_str() {
                            phases.push(p.to_string());
                        }
                    }
                    "error" => {
                        if let Some(m) = event["payload"]["message"].as_str() {
                            errors.push(m.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(ReplaySummary {
        session_id: session_id.to_string(),
        total_events: total,
        phases,
        errors,
    })
}
