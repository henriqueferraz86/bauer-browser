//! RAM monitoring — polls current process + WebView sub-processes via sysinfo.

use sysinfo::{PidExt, ProcessExt, System, SystemExt};

pub struct RamMonitor {
    sys: System,
}

impl RamMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys }
    }

    /// Refresh and return total RAM used by this process and its children (MB).
    pub fn total_mb(&mut self) -> f64 {
        self.sys.refresh_processes();
        let current_pid = std::process::id();
        self.sys
            .processes()
            .iter()
            .filter(|(pid, proc)| {
                let pid_u32 = pid.as_u32();
                pid_u32 == current_pid
                    || proc.parent().map(|p| p.as_u32()) == Some(current_pid)
            })
            .map(|(_, p)| p.memory()) // bytes
            .sum::<u64>() as f64
            / 1_048_576.0 // → MB
    }
}

impl Default for RamMonitor {
    fn default() -> Self {
        Self::new()
    }
}
