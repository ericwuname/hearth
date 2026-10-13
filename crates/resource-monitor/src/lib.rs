// v10.0: Resource monitoring — gives the agent awareness of its own consumption.
use serde::Serialize;
use sysinfo::{Pid, System};

/// Point-in-time snapshot of system and process resources.
///
/// D-206（2026-10-08, traecode）：每个**未知**度量以 `None` 表达（JSON `null`），**不用 `0` 冒充**。
/// `0` 是合法测量值（如"磁盘 0GB 可用"＝全满），拿它表示"探测失败"会把"未知"读成"告警"，
/// 并污染下游聚合/均值（业界口径：未知度量应为 `null` 并从聚合中排除）。与 D-46/D-176 同族。
#[derive(Debug, Clone, Serialize)]
pub struct ResourceSnapshot {
    pub timestamp: String,
    /// 全核平均 CPU 占用；取不到 CPU 列表时为 `None`。
    pub cpu_percent: Option<f32>,
    /// 本进程 RSS（MB）；进程查不到时为 `None`。
    pub memory_mb: Option<f64>,
    /// 全机内存占用百分比；总内存不可得（`<= 0`）时为 `None`。
    pub memory_percent: Option<f32>,
    /// 根盘可用空间（GB）；找不到根盘时为 `None`（**不是 `0`**）。
    pub disk_free_gb: Option<f64>,
    /// 根盘总空间（GB）；找不到根盘时为 `None`（**不是 `0`**）。
    pub disk_total_gb: Option<f64>,
    pub process_pid: u32,
}

/// Collect current resource snapshot.
pub fn snapshot(pid: u32) -> ResourceSnapshot {
    let mut sys = System::new_all();
    sys.refresh_all();

    // D-206：无 CPU 列表 ⇒ 未知（`None`），而非 0/NaN。
    let cpu = {
        let cpus = sys.cpus();
        if cpus.is_empty() {
            None
        } else {
            Some(cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32)
        }
    };

    let mem_used = sys.used_memory() as f64 / 1_048_576.0;
    let mem_total = sys.total_memory() as f64 / 1_048_576.0;
    // D-206：总内存不可得 ⇒ 百分比未知（`None`），而非 0。
    let mem_pct = if mem_total > 0.0 {
        Some((mem_used / mem_total * 100.0) as f32)
    } else {
        None
    };

    // Disk info from sysinfo Disks API
    let disks = sysinfo::Disks::new_with_refreshed_list();
    // D-206：找不到根盘 ⇒ 磁盘可用/总量**未知**（`None`），而非 `(0.0, 0.0)`
    // （后者会被下游读成"磁盘 0GB 可用"并计入告警/均值）。
    let root = disks.iter().find(|d| {
        d.mount_point().to_string_lossy() == "/" || d.mount_point().to_string_lossy() == "C:\\"
    });
    let (df, dt) = match root {
        Some(root) => (
            Some(root.available_space() as f64 / 1_073_741_824.0),
            Some(root.total_space() as f64 / 1_073_741_824.0),
        ),
        None => (None, None),
    };

    // D-206：进程查不到 ⇒ 内存**未知**（`None`），而非 `.unwrap_or(0.0)`。
    let proc_mem = sys
        .process(Pid::from_u32(pid))
        .map(|p| p.memory() as f64 / 1_048_576.0);

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
