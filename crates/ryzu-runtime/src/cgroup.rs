//! cgroup / Docker stats enforcement beyond static caps (M3.2.4).

use crate::config::ResourceLimits;

#[cfg(feature = "docker")]
use bollard::container::Stats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceViolation {
    Memory,
    Cpu,
}

pub fn memory_limit_bytes(limits: &ResourceLimits) -> u64 {
    (limits.memory_mb as u64) * 1024 * 1024
}

/// Usage in bytes from a single Docker stats sample (Bollard reports `u64`).
pub fn memory_usage_bytes(usage: Option<u64>) -> Option<u64> {
    usage
}

pub fn exceeds_memory_limit(usage_bytes: u64, limits: &ResourceLimits) -> bool {
    usage_bytes > memory_limit_bytes(limits)
}

#[cfg(feature = "docker")]
pub fn violation_from_stats(stats: &Stats, limits: &ResourceLimits) -> Option<ResourceViolation> {
    let usage = memory_usage_bytes(stats.memory_stats.usage);

    if let Some(bytes) = usage {
        if exceeds_memory_limit(bytes, limits) {
            return Some(ResourceViolation::Memory);
        }
    }

    let cpu = &stats.cpu_stats;
    let total = u64::try_from(cpu.cpu_usage.total_usage).unwrap_or(0);
    if let Some(system) = cpu.system_cpu_usage {
        let system = u64::try_from(system).unwrap_or(0);
        if system > 0 {
            let pct = (total as f64 / system as f64) * 100.0;
            if pct > limits.cpu_percent as f64 * 1.1 {
                return Some(ResourceViolation::Cpu);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_threshold() {
        let limits = ResourceLimits {
            memory_mb: 256,
            cpu_percent: 50,
        };
        assert!(!exceeds_memory_limit(256 * 1024 * 1024, &limits));
        assert!(exceeds_memory_limit(256 * 1024 * 1024 + 1, &limits));
    }
}
