//! Telemetry module for AION.
//!
//! Phase 1 goal: define *shapes* of metrics and a trait that AION can use,
//! without committing yet to real OS integration.
//!
//! We ship two providers:
//! - SimulatedTelemetry  (default in development)
//! - RealTelemetry       (optional, via env var)
//!
//! The kernel chooses the provider in run_loop().

#![allow(dead_code)]

use std::time::Instant;

/// CPU / GPU related metrics.
#[derive(Debug, Clone, Copy)]
pub struct CpuGpuMetrics {
    pub cpu_load: f32,       // 0..1 normalized
    pub cpu_temp_c: f32,     // degrees C
    pub throttling_events: u32,
    pub gpu_load: f32,       // 0..1
    pub gpu_mem_util: f32,   // 0..1
}

/// Memory / storage related metrics.
#[derive(Debug, Clone, Copy)]
pub struct MemoryMetrics {
    pub ram_used_ratio: f32,     // 0..1
    pub swap_used_ratio: f32,    // 0..1
    pub major_page_faults: f32,  // placeholder
    pub disk_latency_ms: f32,    // placeholder
}

/// IO and network related metrics.
#[derive(Debug, Clone, Copy)]
pub struct IoMetrics {
    pub net_packet_loss: f32, // 0..1
    pub net_latency_ms: f32,
    pub io_queue_depth: f32,  // 0..1
    pub io_error_rate: f32,   // 0..1
}

/// Simulation aggressiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimLevel {
    Off,
    Low,
    High,
}

/// General interface for anything that supplies telemetry.
pub trait TelemetryProvider: Send {
    fn read_cpu_gpu_metrics(&mut self) -> CpuGpuMetrics;
    fn read_memory_metrics(&mut self) -> MemoryMetrics;
    fn read_io_metrics(&mut self) -> IoMetrics;
}

/// ---------------------------------------------------------------------------
/// SIMULATED PROVIDER
/// ---------------------------------------------------------------------------
pub mod sim {
    use super::{CpuGpuMetrics, IoMetrics, MemoryMetrics, SimLevel, TelemetryProvider};

    pub struct SimulatedTelemetry {
        tick: u64,
        pub level: SimLevel,
    }

    impl SimulatedTelemetry {
        pub fn new(level: SimLevel) -> Self {
            Self { tick: 0, level }
        }

        fn next_phase(&mut self) -> f32 {
            self.tick = self.tick.wrapping_add(1);
            (self.tick % 60) as f32 / 60.0
        }
    }

    impl TelemetryProvider for SimulatedTelemetry {
        fn read_cpu_gpu_metrics(&mut self) -> CpuGpuMetrics {
            let p = self.next_phase();
            match self.level {
                SimLevel::Off => CpuGpuMetrics {
                    cpu_load: 0.15,
                    cpu_temp_c: 45.0,
                    throttling_events: 0,
                    gpu_load: 0.10,
                    gpu_mem_util: 0.08,
                },
                SimLevel::Low => CpuGpuMetrics {
                    cpu_load: 0.2 + 0.25 * (p - 0.5).abs(),
                    cpu_temp_c: 45.0 + p * 10.0,
                    throttling_events: 0,
                    gpu_load: 0.15 + 0.2 * p,
                    gpu_mem_util: 0.10 + 0.15 * (1.0 - p),
                },
                SimLevel::High => {
                    let cpu_temp = 55.0 + p * 25.0;
                    CpuGpuMetrics {
                        cpu_load: 0.4 + 0.5 * p,
                        cpu_temp_c: cpu_temp,
                        throttling_events: if cpu_temp > 75.0 { 1 } else { 0 },
                        gpu_load: 0.5 + 0.45 * (1.0 - p),
                        gpu_mem_util: 0.4 + 0.4 * p,
                    }
                }
            }
        }

        fn read_memory_metrics(&mut self) -> MemoryMetrics {
            let p = self.next_phase();
            match self.level {
                SimLevel::Off => MemoryMetrics {
                    ram_used_ratio: 0.3,
                    swap_used_ratio: 0.0,
                    major_page_faults: 0.0,
                    disk_latency_ms: 2.0,
                },
                SimLevel::Low => MemoryMetrics {
                    ram_used_ratio: 0.35 + 0.15 * p,
                    swap_used_ratio: 0.0,
                    major_page_faults: 0.5,
                    disk_latency_ms: 3.0 + 2.0 * p,
                },
                SimLevel::High => MemoryMetrics {
                    ram_used_ratio: 0.6 + 0.35 * p,
                    swap_used_ratio: 0.0,
                    major_page_faults: 2.0 + 5.0 * p,
                    disk_latency_ms: 5.0 + 12.0 * p,
                },
            }
        }

        fn read_io_metrics(&mut self) -> IoMetrics {
            IoMetrics {
                net_packet_loss: 0.0,
                net_latency_ms: 5.0,
                io_queue_depth: 0.1,
                io_error_rate: 0.0,
            }
        }
    }
}

/// ---------------------------------------------------------------------------
/// REAL PROVIDER (Phase 1.2)
/// ---------------------------------------------------------------------------
pub mod real {
    use super::{CpuGpuMetrics, IoMetrics, MemoryMetrics, SimLevel, TelemetryProvider};
    use sysinfo::System;

    pub struct RealTelemetry {
        sys: System,
        _level: SimLevel,
    }

    impl RealTelemetry {
        pub fn new(level: SimLevel) -> Self {
            let sys = System::new_all();
            Self { sys, _level: level }
        }

        fn refresh(&mut self) {
            self.sys.refresh_memory();
        }
    }

    impl TelemetryProvider for RealTelemetry {
        fn read_cpu_gpu_metrics(&mut self) -> CpuGpuMetrics {
            CpuGpuMetrics {
                cpu_load: 0.30,
                cpu_temp_c: 50.0,
                throttling_events: 0,
                gpu_load: 0.0,
                gpu_mem_util: 0.0,
            }
        }

        fn read_memory_metrics(&mut self) -> MemoryMetrics {
            self.refresh();
            let total = self.sys.total_memory() as f32;
            let used  = self.sys.used_memory() as f32;
            let swap_t = self.sys.total_swap() as f32;
            let swap_u = self.sys.used_swap() as f32;

            MemoryMetrics {
                ram_used_ratio: if total > 0.0 { (used/total).clamp(0.0,1.0) } else { 0.0 },
                swap_used_ratio: if swap_t > 0.0 { (swap_u/swap_t).clamp(0.0,1.0) } else { 0.0 },
                major_page_faults: 0.0,
                disk_latency_ms: 5.0,
            }
        }

        fn read_io_metrics(&mut self) -> IoMetrics {
            IoMetrics {
                net_packet_loss: 0.0,
                net_latency_ms: 5.0,
                io_queue_depth: 0.1,
                io_error_rate: 0.0,
            }
        }
    }
}

/// ---------------------------------------------------------------------------
/// Health computation utilities
/// ---------------------------------------------------------------------------

fn clamp01(x: f32) -> f32 {
    x.max(0.0).min(1.0)
}

pub fn compute_cortex_health(m: &CpuGpuMetrics) -> f32 {
    let temp_penalty = if m.cpu_temp_c <= 60.0 {
        0.0
    } else {
        ((m.cpu_temp_c - 60.0) / 40.0).min(0.6)
    };
    clamp01(1.0 - temp_penalty)
}

pub fn compute_memory_health(m: &MemoryMetrics) -> f32 {
    let ram_penalty = if m.ram_used_ratio <= 0.75 {
        0.0
    } else {
        (m.ram_used_ratio - 0.75).min(0.3)
    };
    clamp01(1.0 - ram_penalty)
}

pub fn compute_iobridge_health(m: &IoMetrics) -> f32 {
    let loss_penalty = (m.net_packet_loss * 4.0).min(0.4);
    clamp01(1.0 - loss_penalty)
}
