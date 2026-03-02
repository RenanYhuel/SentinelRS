use sysinfo::System;

use sentinel_common::proto::SystemStats;

pub fn collect_system_stats() -> SystemStats {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks.iter().fold((0u64, 0u64), |(u, t), d| {
        (
            u + d.total_space() - d.available_space(),
            t + d.total_space(),
        )
    });

    let load_avg = System::load_average();

    SystemStats {
        cpu_percent: sys.global_cpu_usage() as f64,
        memory_used_bytes: sys.used_memory(),
        memory_total_bytes: sys.total_memory(),
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
        load_avg_1m: load_avg.one,
        process_count: sys.processes().len() as u32,
        uptime_seconds: System::uptime(),
        os_name: System::long_os_version().unwrap_or_default(),
        hostname: System::host_name().unwrap_or_default(),
    }
}
