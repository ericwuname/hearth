// v10.0: Resource monitoring — gives the agent awareness of its own consumption.
use serde::Serialize;
use sysinfo::{Pid, System};

/// Point-in-time snapshot of system and process resources.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceSnapshot {
    pub timestamp: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
    pub memory_percent: f32,
    pub disk_free_gb: f64,
    pub disk_total_gb: f64,
    pub process_pid: u32,
}

/// Collect current resource snapshot.
pub fn snapshot(pid: u32) -> ResourceSnapshot {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;

    let mem_used = sys.used_memory() as f64 / 1_048_576.0;
    let mem_total = sys.total_memory() as f64 / 1_048_576.0;
    let mem_pct = if mem_total > 0.0 {
        (mem_used / mem_total * 100.0) as f32
    } else {
        0.0
    };

    // Disk info from sysinfo Disks API
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let (df, dt) = if let Some(root) = disks.iter().find(|d| {
        d.mount_point().to_string_lossy() == "/" || d.mount_point().to_string_lossy() == "C:\\"
    }) {
        (
            root.available_space() as f64 / 1_073_741_824.0,
            root.total_space() as f64 / 1_073_741_824.0,
        )
    } else {
        (0.0, 0.0)
    };

    let proc = sys.process(Pid::from_u32(pid));
    let proc_mem = proc.map(|p| p.memory() as f64 / 1_048_576.0).unwrap_or(0.0);

    ResourceSnapshot {
        timestamp: chrono::Utc::now().to_rfc3339(),
        cpu_percent: cpu,
        memory_mb: proc_mem,
        memory_percent: mem_pct,
        disk_free_gb: df,
        disk_total_gb: dt,
        process_pid: pid,
    }
}

// ── v10.1: ROI & critical resource checks ──

#[derive(Debug, Clone, Serialize)]
pub struct RoiReport {
    pub session_id: String,
    pub tokens_total: u64,
    pub cost_est_usd: f64,
    pub steps: u32,
    pub roi_score: f64,
}

pub fn compute_roi(session_id: &str, tokens: u64, steps: u32, lines_changed: u32) -> RoiReport {
    let cost = (tokens as f64) * 0.000_002;
    let roi = if cost > 0.001 {
        ((lines_changed as f64) / cost).min(1.0)
    } else {
        1.0
    };
    RoiReport {
        session_id: session_id.into(),
        tokens_total: tokens,
        cost_est_usd: (cost * 1000.0).round() / 1000.0,
        steps,
        roi_score: (roi * 1000.0).round() / 1000.0,
    }
}

impl ResourceSnapshot {
    pub fn is_critical(&self) -> bool {
        self.memory_percent > 80.0 || self.disk_free_gb < 1.0
    }
}

#[cfg(test)]
mod roi_tests {
    use super::*;
    #[test]
    fn test_roi() {
        assert!(compute_roi("s1", 10000, 5, 200).roi_score > 0.0);
    }
    #[test]
    fn test_critical() {
        let mut s = snapshot(std::process::id());
        s.memory_percent = 85.0;
        assert!(s.is_critical());
    }
}
