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

// ── v10.1 死代码清理（D-71，2026-10-01, traecode）──
//
// 原 v10.1 计划的 `RoiReport` / `compute_roi`（`GET /api/v1/costs/roi`）与
// `ResourceSnapshot::is_critical` **从未接线**：全仓零生产调用方（仅自测调用，
// v10.1 计划落空；`docs/global-panorama-v10.1.md:79` 曾如实记录"仅自测调用"）。
// 且 `compute_roi` 用**硬编码假价** `tokens * 0.000002` 估算成本——与 D-46
// 确立的口径（"未知价 → 显式不可用，绝不按 0/粗略值冒充"）正面冲突，故不订正
// 而是**删除**。真实成本核算的唯一事实源现为 `llm-gateway` 的 `PriceTable` +
// `CostMeter`（经 `nervous-system::set_cost` 接入）。
//
// `ResourceSnapshot` 本身仍现役：`GET /api/v1/resources`（service/routes.rs）与
// 启动自检（service/main.rs）都消费它。
