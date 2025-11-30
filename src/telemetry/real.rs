//! Real telemetry provider using the `sysinfo` crate.
//!
//! Phase 1: keep this extremely conservative so it compiles cleanly
//! with sysinfo 0.30.x on your machine. We mostly use `sysinfo` for
//! basic memory stats and return safe placeholder values for the rest.
//!
//! The actual kernel is still wired to `SimulatedTelemetry`; this
//! `RealTelemetry` is here so we can switch over later without
//! touching the rest of the system.

use sysinfo::System;

use super::{
    CpuGpuMetrics,
    IoMetrics,
    MemoryMetrics,
    SimLevel,
    TelemetryProvider,
};

/// Telemetry backed by the host OS via `sysinfo`.
///
/// Right now this is deliberately minimal:
/// - CPU/GPU values are placeholders.
/// - Memory values use real `sysinfo` totals/used.
/// - IO/network values are placeholders.
///
/// That keeps the file compiling across sysinfo 0.30.x without
/// depending on any of the newer / more complex APIs.
pub struct RealTelemetry {
    sys: System,
    _level: SimLevel,
}

impl RealTelemetry {
    /// Create a new real telemetry provider.
    pub fn new(level: SimLevel) -> Self {
        let sys = System::new_all();
        Self { sys, _level: level }
    }

    /// Refresh the bits we actually read.
    fn refresh(&mut self) {
        self.sys.refresh_memory();
    }
}

impl TelemetryProvider for RealTelemetry {
    // -------------------------------------------------------------------------
    // CPU + GPU (currently stubbed)
    // -------------------------------------------------------------------------
    fn read_cpu_gpu_metrics(&mut self) -> CpuGpuMetrics {
        // For now we don't rely on any sysinfo CPU APIs at all here,
        // to avoid version/feature differences. Just return a stable,
        // "healthy-ish" placeholder.
        CpuGpuMetrics {
            cpu_load: 0.30,         // 30% load
            cpu_temp_c: 50.0,       // 50°C nominal
            throttling_events: 0,   // unknown → assume none
            gpu_load: 0.0,          // no GPU integration yet
            gpu_mem_util: 0.0,
        }
    }

    // -------------------------------------------------------------------------
    // MEMORY + SWAP (real numbers from sysinfo)
    // -------------------------------------------------------------------------
    fn read_memory_metrics(&mut self) -> MemoryMetrics {
        self.refresh();

        let total_mem = self.sys.total_memory() as f32;
        let used_mem  = self.sys.used_memory() as f32;
        let total_swap = self.sys.total_swap() as f32;
        let used_swap  = self.sys.used_swap() as f32;

        let ram_used_ratio = if total_mem > 0.0 {
            (used_mem / total_mem).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let swap_used_ratio = if total_swap > 0.0 {
            (used_swap / total_swap).clamp(0.0, 1.0)
        } else {
            0.0
        };

        MemoryMetrics {
            ram_used_ratio,
            swap_used_ratio,
            major_page_faults: 0.0, // not available via simple sysinfo
            disk_latency_ms: 5.0,   // neutral placeholder
        }
    }

    // -------------------------------------------------------------------------
    // IO + NETWORK (currently stubbed)
    // -------------------------------------------------------------------------
    fn read_io_metrics(&mut self) -> IoMetrics {
        // Same idea as CPU: safe "healthy" defaults so IoBridge health
        // stays near 1.0 unless the sim layer pushes it around.
        IoMetrics {
            net_packet_loss: 0.0,
            net_latency_ms: 5.0,
            io_queue_depth: 0.1,
            io_error_rate: 0.0,
        }
    }
}
