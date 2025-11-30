use std::fs;
use std::io::{self, BufRead, Write};
use std::process;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::http::HttpServer;
use crate::memory::{MemoryBus, MemoryScope};
use crate::organism::{
    self, format_topology_brief, Organ, OrganKind, SystemTopology,
};
use crate::telemetry::{
    self, TelemetryProvider, SimLevel,
    sim::SimulatedTelemetry,
    real::RealTelemetry,
    CpuGpuMetrics, MemoryMetrics, IoMetrics,
};

/// Different categories of pulses travelling on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseKind {
    Heartbeat,
    Status,
    Command,
    Ai,
    Sim,
}

/// Which telemetry backend is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryMode {
    Simulated,
    Real,
}

/// Log filtering for bus output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    All,
    CommandsOnly,
    Silent,
}

/// Simple message bus. Right now it just logs, but it also
/// tracks global "world" state like awareness and sim level.
/// It now carries a shared MemoryBus (from src/memory).
pub struct Bus {
    next_id: u64,
    pub log_filter: LogFilter,
    pub sim_level: SimLevel,
    pub awareness_score: f32,
    pub telemetry_mode: TelemetryMode,
    pub memory: MemoryBus,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            log_filter: LogFilter::CommandsOnly,
            sim_level: SimLevel::Low,
            awareness_score: 1.0,
            telemetry_mode: TelemetryMode::Simulated,
            memory: MemoryBus::new(),
        }
    }

    pub fn emit(&mut self, kind: PulseKind, source: &'static str, data: impl Into<String>) {
        self.next_id += 1;
        let data = data.into();

        match self.log_filter {
            LogFilter::All => {
                println!(
                    "[BUS][{:?}] pulse#{} from {} => {}",
                    kind, self.next_id, source, data
                );
            }
            LogFilter::CommandsOnly => {
                if kind == PulseKind::Command {
                    println!(
                        "[BUS][{:?}] pulse#{} from {} => {}",
                        kind, self.next_id, source, data
                    );
                }
            }
            LogFilter::Silent => {
                // print nothing
            }
        }
    }
}

pub fn boot() {
    println!("[AION-KERNEL] Boot sequence started.");
    println!("[AION-KERNEL] Initializing messaging bus...");
    println!("[AION-KERNEL] Loading core daemons...");
    println!("[AION-KERNEL] Kernel online.");
}

/// A snapshot of the most recent telemetry as seen by the StatusDaemon.
#[derive(Debug, Clone, Copy)]
pub struct TelemetrySnapshot {
    pub cpu: CpuGpuMetrics,
    pub mem: MemoryMetrics,
    pub io: IoMetrics,
}

/// Basic interface for any long-running kernel task.
pub trait Daemon {
    fn name(&self) -> &'static str;
    fn tick(&mut self, now: Instant, bus: &mut Bus);
}

/// A simple daemon that prints a heartbeat every N milliseconds.
pub struct HeartbeatDaemon {
    last_run: Instant,
    interval: Duration,
    counter: u64,
}

impl HeartbeatDaemon {
    pub fn new(interval: Duration) -> Self {
        Self {
            last_run: Instant::now(),
            interval,
            counter: 0,
        }
    }
}

impl Daemon for HeartbeatDaemon {
    fn name(&self) -> &'static str {
        "heartbeat"
    }

    fn tick(&mut self, now: Instant, bus: &mut Bus) {
        if now.duration_since(self.last_run) >= self.interval {
            self.counter += 1;
            self.last_run = now;

            bus.emit(
                PulseKind::Heartbeat,
                self.name(),
                format!("beat #{}", self.counter),
            );
        }
    }
}

/// A daemon that reports overall system / organism status.
/// In Phase 1 it also uses a TelemetryProvider to gently
/// pull organ health toward values derived from metrics.
pub struct StatusDaemon {
    last_run: Instant,
    interval: Duration,
    counter: u64,
    topology: Arc<Mutex<SystemTopology>>,
    telemetry: Box<dyn TelemetryProvider>,
    /// Shared snapshot for the `metrics` command / HTTP.
    metrics_snapshot: Arc<Mutex<Option<TelemetrySnapshot>>>,
}

impl StatusDaemon {
    pub fn new(
        interval: Duration,
        topology: Arc<Mutex<SystemTopology>>,
        telemetry: Box<dyn TelemetryProvider>,
        metrics_snapshot: Arc<Mutex<Option<TelemetrySnapshot>>>,
    ) -> Self {
        Self {
            last_run: Instant::now(),
            interval,
            counter: 0,
            topology,
            telemetry,
            metrics_snapshot,
        }
    }

    /// Blend current organ health toward a target health (0.0–1.0).
    /// `alpha` controls how fast we move: 0.0 = no change, 1.0 = snap.
    fn blend_health(current: f32, target: f32, alpha: f32) -> f32 {
        let c = current.clamp(0.0, 1.0);
        let t = target.clamp(0.0, 1.0);
        (1.0 - alpha) * c + alpha * t
    }

    fn apply_telemetry_to_topology(
        topology: &mut SystemTopology,
        cpu_gpu: &CpuGpuMetrics,
        mem: &MemoryMetrics,
        io: &IoMetrics,
    ) {
        let target_cortex = telemetry::compute_cortex_health(cpu_gpu);
        let target_memory = telemetry::compute_memory_health(mem);
        let target_iobridge = telemetry::compute_iobridge_health(io);

        let alpha = 0.25; // 25% toward telemetry per status tick

        for organ in &mut topology.organs {
            match organ.kind {
                OrganKind::Cortex => {
                    organ.health = Self::blend_health(organ.health, target_cortex, alpha);
                }
                OrganKind::Memory => {
                    organ.health = Self::blend_health(organ.health, target_memory, alpha);
                }
                OrganKind::IoBridge => {
                    organ.health = Self::blend_health(organ.health, target_iobridge, alpha);
                }
                _ => { /* other organs not wired yet */ }
            }
        }
    }
}

impl Daemon for StatusDaemon {
    fn name(&self) -> &'static str {
        "status"
    }

    fn tick(&mut self, now: Instant, bus: &mut Bus) {
        if now.duration_since(self.last_run) < self.interval {
            return;
        }

        self.counter += 1;
        self.last_run = now;

        // Pull metrics from telemetry.
        let cpu_gpu = self.telemetry.read_cpu_gpu_metrics();
        let mem = self.telemetry.read_memory_metrics();
        let io = self.telemetry.read_io_metrics();

        // Update shared metrics snapshot for the `metrics` command + HTTP.
        if let Ok(mut guard) = self.metrics_snapshot.lock() {
            *guard = Some(TelemetrySnapshot { cpu: cpu_gpu, mem, io });
        }

        let brief;

        if let Ok(mut topo) = self.topology.lock() {
            // Apply telemetry-driven health adjustments.
            Self::apply_telemetry_to_topology(&mut *topo, &cpu_gpu, &mem, &io);

            // Recompute awareness from updated topology.
            let awareness = organism::compute_awareness(&*topo);
            let awareness_label = organism::describe_awareness(awareness);

            brief = format_topology_brief(&*topo);

            let overall_health = compute_overall_health(&*topo);
            let health_label = classify_health(overall_health);

            bus.awareness_score = awareness;

            let msg = format!(
                "status tick #{} :: {} :: health {:.2} ({}) :: awareness {:.2} ({})",
                self.counter, brief, overall_health, health_label, awareness, awareness_label
            );

            // Store the last status line in memory (global scope).
            bus.memory
                .set_text(MemoryScope::Global, "kernel.last_status", msg.clone());

            bus.emit(PulseKind::Status, self.name(), msg);
        } else {
            bus.emit(
                PulseKind::Status,
                self.name(),
                "status tick: failed to lock topology",
            );
        }
    }
}

/// A daemon representing the AI Cortex: all high-level intelligence lives here.
///
/// In this early phase it observes awareness and logs a coarse "policy"
/// about how the system should behave, and writes that decision into the MemoryBus.
pub struct AiDaemon {
    last_run: Instant,
    interval: Duration,
    cycle: u64,
    topology: Arc<Mutex<SystemTopology>>,
}

impl AiDaemon {
    pub fn new(interval: Duration, topology: Arc<Mutex<SystemTopology>>) -> Self {
        Self {
            last_run: Instant::now(),
            interval,
            cycle: 0,
            topology,
        }
    }
}

impl Daemon for AiDaemon {
    fn name(&self) -> &'static str {
        "ai-cortex"
    }

    fn tick(&mut self, now: Instant, bus: &mut Bus) {
        if now.duration_since(self.last_run) < self.interval {
            return;
        }

        self.cycle += 1;
        self.last_run = now;

        let awareness = if let Ok(topo) = self.topology.lock() {
            organism::compute_awareness(&*topo)
        } else {
            bus.awareness_score
        };
        let label = organism::describe_awareness(awareness);

        // Tiny policy brain: decide what we *would* do.
        let policy = if awareness >= 0.85 {
            "policy=push_capacity"          // safe to run heavy workloads
        } else if awareness >= 0.60 {
            "policy=maintain_load"          // keep as is
        } else if awareness >= 0.35 {
            "policy=reduce_load"            // consider reducing sim/load
        } else if awareness > 0.0 {
            // Critical: also force sim_level off as a protective reflex.
            if bus.sim_level != SimLevel::Off {
                bus.sim_level = SimLevel::Off;
            }
            "policy=protect_core(sim_off)"   // emergency mode
        } else {
            "policy=recover_offline"        // unconscious
        };

        // Write policy + awareness into the shared MemoryBus (global scope).
        bus.memory
            .set_text(MemoryScope::Global, "cortex.policy", policy);
        bus.memory
            .set_text(MemoryScope::Global, "cortex.awareness", format!("{:.3}", awareness));
        bus.memory
            .set_text(MemoryScope::Global, "cortex.awareness_label", label);

        let msg = format!(
            "cortex cycle #{} :: awareness {:.2} ({}) :: {}",
            self.cycle, awareness, label, policy
        );

        bus.emit(PulseKind::Ai, self.name(), msg);
    }
}

/// A daemon that simulates environmental pressure / recovery.
/// This is separate from telemetry and purely synthetic, controlled by sim_level.
pub struct SimulationDaemon {
    last_run: Instant,
    interval: Duration,
    tick: u64,
    topology: Arc<Mutex<SystemTopology>>,
}

impl SimulationDaemon {
    pub fn new(interval: Duration, topology: Arc<Mutex<SystemTopology>>) -> Self {
        Self {
            last_run: Instant::now(),
            interval,
            tick: 0,
            topology,
        }
    }

    fn nudge_health(organ: &mut Organ, delta: f32) {
        organ.health = (organ.health + delta).clamp(0.0, 1.0);
    }
}

impl Daemon for SimulationDaemon {
    fn name(&self) -> &'static str {
        "sim"
    }

    fn tick(&mut self, now: Instant, bus: &mut Bus) {
        if now.duration_since(self.last_run) < self.interval {
            return;
        }

        self.last_run = now;
        self.tick = self.tick.wrapping_add(1);

        if bus.sim_level == SimLevel::Off {
            return;
        }

        if let Ok(mut topo) = self.topology.lock() {
            if topo.organs.is_empty() {
                return;
            }

            let idx = (self.tick % topo.organs.len() as u64) as usize;
            let organ = &mut topo.organs[idx];

            let (delta, label) = match bus.sim_level {
                SimLevel::Low => {
                    // Mostly small negative hits, occasional recovery.
                    if self.tick % 5 == 0 {
                        (0.02, "recovery")
                    } else {
                        (-0.01, "stress")
                    }
                }
                SimLevel::High => {
                    if self.tick % 3 == 0 {
                        (0.03, "recovery")
                    } else {
                        (-0.04, "stress")
                    }
                }
                SimLevel::Off => {
                    return;
                }
            };

            Self::nudge_health(organ, delta);

            let msg = format!(
                "{} tick on {:?}: health now {:.2}",
                label, organ.kind, organ.health
            );
            bus.emit(PulseKind::Sim, self.name(), msg);
        }
    }
}

/// A daemon that processes user commands from stdin.
/// This is the first AION "shell" interface.
pub struct CommandDaemon {
    rx: Receiver<String>,
    topology: Arc<Mutex<SystemTopology>>,
    metrics_snapshot: Arc<Mutex<Option<TelemetrySnapshot>>>,
}

impl CommandDaemon {
    pub fn new(
        rx: Receiver<String>,
        topology: Arc<Mutex<SystemTopology>>,
        metrics_snapshot: Arc<Mutex<Option<TelemetrySnapshot>>>,
    ) -> Self {
        Self {
            rx,
            topology,
            metrics_snapshot,
        }
    }

    fn parse_organ_kind(name: &str) -> Option<OrganKind> {
        match name.to_lowercase().as_str() {
            "cortex" => Some(OrganKind::Cortex),
            "memory" => Some(OrganKind::Memory),
            "iobridge" | "io" => Some(OrganKind::IoBridge),
            "sensorhub" => Some(OrganKind::SensorHub),
            "motorcontrol" | "motor" => Some(OrganKind::MotorControl),
            "network" => Some(OrganKind::Network),
            "storage" => Some(OrganKind::Storage),
            _ => None,
        }
    }

    fn organ_health_report(topology: &SystemTopology) -> String {
        let mut out = String::new();
        out.push_str("Organ health:\n");
        for organ in &topology.organs {
            let label = classify_health(organ.health);
            out.push_str(&format!(" - {:?}: {:.2} ({})\n", organ.kind, organ.health, label));
        }
        out
    }

    fn alerts_report(topology: &SystemTopology) -> String {
        let mut out = String::new();
        out.push_str("Alerts:\n");

        let mut any = false;
        let mut min_health: f32 = 1.0;

        for organ in &topology.organs {
            min_health = min_health.min(organ.health);
            let label = classify_health(organ.health);
            if label != "ok" {
                any = true;
                out.push_str(&format!(" - {:?}: {:.2} [{}]\n", organ.kind, organ.health, label));
            }
        }

        if !any {
            out.push_str(" (no active alerts; all organs healthy)\n");
        } else {
            out.push_str(&format!("overall: {}\n", classify_health(min_health)));
        }

        out
    }

    fn sim_status_report(topology: &SystemTopology, bus: &Bus) -> String {
        let min_health = compute_overall_health(topology);
        let awareness = bus.awareness_score;
        format!(
            "simulation status: level={:?} :: min health {:.2} :: awareness {:.2}",
            bus.sim_level, min_health, awareness
        )
    }

    fn handle_damage(
        &self,
        parts: &[&str],
        bus: &mut Bus,
    ) -> Option<String> {
        if parts.len() != 3 {
            return Some("usage: damage <organ> <amount>".to_string());
        }
        let organ_name = parts[1];
        let amount_str = parts[2];
        let amount: f32 = match amount_str.parse() {
            Ok(v) => v,
            Err(_) => {
                return Some(format!("invalid amount: {}", amount_str));
            }
        };

        let kind = match Self::parse_organ_kind(organ_name) {
            Some(k) => k,
            None => {
                return Some(format!("unknown organ '{}'", organ_name));
            }
        };

        if let Ok(mut topo) = self.topology.lock() {
            let mut new_health = None;
            for organ in &mut topo.organs {
                if organ.kind == kind {
                    organ.health = (organ.health - amount).clamp(0.0, 1.0);
                    new_health = Some(organ.health);
                    break;
                }
            }
            if let Some(h) = new_health {
                let awareness = organism::compute_awareness(&*topo);
                bus.awareness_score = awareness;
                let label = organism::describe_awareness(awareness);
                return Some(format!(
                    "damaged {:?} by {:.2}, new health {:.2} (awareness {:.2} {})",
                    kind, amount, h, awareness, label
                ));
            } else {
                return Some(format!("organ {:?} not found in topology", kind));
            }
        } else {
            Some("failed to lock topology for damage".to_string())
        }
    }

    fn handle_heal(
        &self,
        parts: &[&str],
        bus: &mut Bus,
    ) -> Option<String> {
        if parts.len() != 3 {
            return Some("usage: heal <organ> <amount>".to_string());
        }
        let organ_name = parts[1];
        let amount_str = parts[2];
        let amount: f32 = match amount_str.parse() {
            Ok(v) => v,
            Err(_) => {
                return Some(format!("invalid amount: {}", amount_str));
            }
        };

        let kind = match Self::parse_organ_kind(organ_name) {
            Some(k) => k,
            None => {
                return Some(format!("unknown organ '{}'", organ_name));
            }
        };

        if let Ok(mut topo) = self.topology.lock() {
            let mut new_health = None;
            for organ in &mut topo.organs {
                if organ.kind == kind {
                    organ.health = (organ.health + amount).clamp(0.0, 1.0);
                    new_health = Some(organ.health);
                    break;
                }
            }
            if let Some(h) = new_health {
                let awareness = organism::compute_awareness(&*topo);
                bus.awareness_score = awareness;
                let label = organism::describe_awareness(awareness);
                return Some(format!(
                    "healed {:?} by {:.2}, new health {:.2} (awareness {:.2} {})",
                    kind, amount, h, awareness, label
                ));
            } else {
                return Some(format!("organ {:?} not found in topology", kind));
            }
        } else {
            Some("failed to lock topology for heal".to_string())
        }
    }

    fn handle_save_state(&self, _bus: &mut Bus) -> String {
        if let Ok(topo) = self.topology.lock() {
            let mut lines = Vec::new();
            for organ in &topo.organs {
                lines.push(format!("{:?} {:.5}", organ.kind, organ.health));
            }
            match fs::write("aion_state.txt", lines.join("\n")) {
                Ok(_) => "state saved to aion_state.txt".to_string(),
                Err(e) => format!("failed to save state: {}", e),
            }
        } else {
            "failed to lock topology for save".to_string()
        }
    }

    fn handle_load_state(&self, bus: &mut Bus) -> String {
        let content = match fs::read_to_string("aion_state.txt") {
            Ok(c) => c,
            Err(e) => return format!("failed to load state: {}", e),
        };

        if let Ok(mut topo) = self.topology.lock() {
            for line in content.lines() {
                let mut parts = line.split_whitespace();
                let kind_str = match parts.next() {
                    Some(k) => k,
                    None => continue,
                };
                let health_str = match parts.next() {
                    Some(h) => h,
                    None => continue,
                };

                let kind = match Self::parse_organ_kind(kind_str) {
                    Some(k) => k,
                    None => continue,
                };

                let h: f32 = match health_str.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                for organ in &mut topo.organs {
                    if organ.kind == kind {
                        organ.health = h.clamp(0.0, 1.0);
                        break;
                    }
                }
            }

            let awareness = organism::compute_awareness(&*topo);
            bus.awareness_score = awareness;
            let label = organism::describe_awareness(awareness);
            format!(
                "state loaded from aion_state.txt (awareness {:.2} {})",
                awareness, label
            )
        } else {
            "failed to lock topology for load".to_string()
        }
    }

    fn handle_mem(parts: &[&str], bus: &mut Bus) -> Option<String> {
        // mem / mem ls – list everything (text dump)
        if parts.len() == 1 || (parts.len() == 2 && parts[1] == "ls") {
            return Some(bus.memory.dump());
        }

        // mem get <key>
        if parts.len() >= 3 && parts[1] == "get" {
            let key = parts[2];
            match bus.memory.get(MemoryScope::Global, key) {
                Some(v) => Some(format!("mem[{}] = {}", key, v)),
                None => Some(format!("mem: key '{}' not found", key)),
            }
        }
        // mem set <key> <value>
        else if parts.len() >= 4 && parts[1] == "set" {
            let key = parts[2];
            let value = parts[3..].join(" ");
            bus.memory.set_text(MemoryScope::Global, key, value);
            Some(format!("mem[{}] updated", key))
        } else {
            Some(
                "usage: mem [ls] | mem get <key> | mem set <key> <value>".to_string(),
            )
        }
    }
}

impl Daemon for CommandDaemon {
    fn name(&self) -> &'static str {
        "command"
    }

    fn tick(&mut self, _now: Instant, bus: &mut Bus) {
        loop {
            match self.rx.try_recv() {
                Ok(cmd) => {
                    let trimmed = cmd.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let parts: Vec<&str> = trimmed.split_whitespace().collect();

                    let response = match parts[0] {
                        "help" => Some(
                            "commands: help, status, topology, nodes, organs, peripherals, health, \
                             awareness, metrics, mode, alerts, sim status, sim level <off|low|high>, \
                             mem, mem get <key>, mem set <key> <value>, \
                             save state, load state, damage <organ> <amount>, heal <organ> <amount>, \
                             logs all, logs commands, logs silent, quit"
                                .to_string(),
                        ),

                        "status" => {
                            if let Ok(topo) = self.topology.lock() {
                                let brief = format_topology_brief(&*topo);
                                let overall_health = compute_overall_health(&*topo);
                                let health_label = classify_health(overall_health);
                                let awareness = bus.awareness_score;
                                let awareness_label = organism::describe_awareness(awareness);
                                Some(format!(
                                    "manual status :: {} :: health {:.2} ({}) :: awareness {:.2} ({})",
                                    brief, overall_health, health_label, awareness, awareness_label
                                ))
                            } else {
                                Some("failed to lock topology".to_string())
                            }
                        }

                        "topology" => {
                            if let Ok(topo) = self.topology.lock() {
                                let mut details = String::new();
                                details.push_str("Topology detail:\n");
                                for node in &topo.nodes {
                                    details.push_str(&format!(
                                        " - Node {} [{}]: {}\n",
                                        node.id.0, node.label, node.role
                                    ));
                                }
                                for organ in &topo.organs {
                                    details.push_str(&format!(
                                        "   - Organ {:?} on Node {} (health {:.2})\n",
                                        organ.kind, organ.node.0, organ.health
                                    ));
                                }
                                Some(details)
                            } else {
                                Some("failed to lock topology".to_string())
                            }
                        }

                        "nodes" => {
                            if let Ok(topo) = self.topology.lock() {
                                let mut details = String::new();
                                details.push_str("Nodes:\n");
                                for node in &topo.nodes {
                                    details.push_str(&format!(
                                        " - Node {} [{}]: {}\n",
                                        node.id.0, node.label, node.role
                                    ));
                                }
                                Some(details)
                            } else {
                                Some("failed to lock topology".to_string())
                            }
                        }

                        "organs" => {
                            if let Ok(topo) = self.topology.lock() {
                                let mut details = String::new();
                                details.push_str("Organs:\n");
                                for organ in &topo.organs {
                                    details.push_str(&format!(
                                        " - Organ {:?} on Node {} (health {:.2})\n",
                                        organ.kind, organ.node.0, organ.health
                                    ));
                                }
                                Some(details)
                            } else {
                                Some("failed to lock topology".to_string())
                            }
                        }

                        "peripherals" => {
                            if let Ok(topo) = self.topology.lock() {
                                let mut details = String::new();
                                details.push_str("Peripherals by organ:\n");
                                for organ in &topo.organs {
                                    if organ.peripherals.is_empty() {
                                        continue;
                                    }
                                    details.push_str(&format!(" - Organ {:?}:\n", organ.kind));
                                    for p in &organ.peripherals {
                                        details.push_str(&format!("    - {:?}: {}\n", p.kind, p.name));
                                    }
                                }
                                if !details.contains("Organ") {
                                    details.push_str(" (no peripherals registered)\n");
                                }
                                Some(details)
                            } else {
                                Some("failed to lock topology".to_string())
                            }
                        }

                        "health" => {
                            if let Ok(topo) = self.topology.lock() {
                                Some(Self::organ_health_report(&*topo))
                            } else {
                                Some("failed to lock topology for health".to_string())
                            }
                        }

                        "awareness" => {
                            if let Ok(topo) = self.topology.lock() {
                                let awareness = organism::compute_awareness(&*topo);
                                let label = organism::describe_awareness(awareness);
                                Some(format!("awareness index: {:.2} :: {}", awareness, label))
                            } else {
                                Some("failed to lock topology for awareness".to_string())
                            }
                        }

                        "alerts" => {
                            if let Ok(topo) = self.topology.lock() {
                                Some(Self::alerts_report(&*topo))
                            } else {
                                Some("failed to lock topology for alerts".to_string())
                            }
                        }

                        "mode" => {
                            let tele_str = match bus.telemetry_mode {
                                TelemetryMode::Simulated => "simulated",
                                TelemetryMode::Real => "real",
                            };
                            Some(format!(
                                "mode :: telemetry={} :: sim_level={:?}",
                                tele_str, bus.sim_level
                            ))
                        }

                        "metrics" => {
                            match self.metrics_snapshot.lock() {
                                Ok(guard) => {
                                    if let Some(snap) = *guard {
                                        let mut out = String::new();
                                        out.push_str("Metrics snapshot (from status daemon):\n");
                                        out.push_str(" Cortex / CPU+GPU:\n");
                                        out.push_str(&format!(
                                            "  cpu_load      : {:.2}\n  cpu_temp_c    : {:.1}\n  throttling    : {}\n  gpu_load      : {:.2}\n  gpu_mem_util  : {:.2}\n",
                                            snap.cpu.cpu_load,
                                            snap.cpu.cpu_temp_c,
                                            snap.cpu.throttling_events,
                                            snap.cpu.gpu_load,
                                            snap.cpu.gpu_mem_util,
                                        ));
                                        out.push_str(" Memory:\n");
                                        out.push_str(&format!(
                                            "  ram_used      : {:.2}\n  swap_used     : {:.2}\n  page_faults   : {:.1}\n  disk_latency  : {:.1} ms\n",
                                            snap.mem.ram_used_ratio,
                                            snap.mem.swap_used_ratio,
                                            snap.mem.major_page_faults,
                                            snap.mem.disk_latency_ms,
                                        ));
                                        out.push_str(" IoBridge / IO+Net:\n");
                                        out.push_str(&format!(
                                            "  net_loss      : {:.3}\n  net_latency   : {:.1} ms\n  io_queue      : {:.2}\n  io_error_rate : {:.3}\n",
                                            snap.io.net_packet_loss,
                                            snap.io.net_latency_ms,
                                            snap.io.io_queue_depth,
                                            snap.io.io_error_rate,
                                        ));
                                        Some(out)
                                    } else {
                                        Some(
                                            "metrics not yet available (status daemon has not produced a snapshot)"
                                                .to_string(),
                                        )
                                    }
                                }
                                Err(_) => Some("failed to lock metrics snapshot".to_string()),
                            }
                        }

                        "sim" if parts.len() > 1 && parts[1] == "status" => {
                            if let Ok(topo) = self.topology.lock() {
                                Some(Self::sim_status_report(&*topo, bus))
                            } else {
                                Some("failed to lock topology for sim status".to_string())
                            }
                        }

                        "sim" if parts.len() > 2 && parts[1] == "level" => {
                            let level_str = parts[2].to_lowercase();
                            match level_str.as_str() {
                                "off" => {
                                    bus.sim_level = SimLevel::Off;
                                    Some("simulation level set to off".to_string())
                                }
                                "low" => {
                                    bus.sim_level = SimLevel::Low;
                                    Some("simulation level set to low".to_string())
                                }
                                "high" => {
                                    bus.sim_level = SimLevel::High;
                                    Some("simulation level set to high".to_string())
                                }
                                _ => Some(
                                    "usage: sim level <off|low|high>".to_string(),
                                ),
                            }
                        }

                        "mem" => Self::handle_mem(&parts, bus),

                        "damage" => self.handle_damage(&parts, bus),
                        "heal" => self.handle_heal(&parts, bus),

                        "save" if parts.len() > 1 && parts[1] == "state" => {
                            Some(self.handle_save_state(bus))
                        }

                        "load" if parts.len() > 1 && parts[1] == "state" => {
                            Some(self.handle_load_state(bus))
                        }

                        "logs" if parts.len() > 1 && parts[1] == "all" => {
                            bus.log_filter = LogFilter::All;
                            Some("logging: ALL pulses".to_string())
                        }
                        "logs" if parts.len() > 1 && parts[1] == "commands" => {
                            bus.log_filter = LogFilter::CommandsOnly;
                            Some("logging: COMMANDS ONLY".to_string())
                        }
                        "logs" if parts.len() > 1 && (parts[1] == "silent" || parts[1] == "off") => {
                            bus.log_filter = LogFilter::Silent;
                            Some("logging: SILENT".to_string())
                        }

                        "quit" => {
                            Some("shutting down kernel (process::exit(0))".to_string())
                        }

                        _ => Some(format!("unknown command: '{}'", trimmed)),
                    };

                    if let Some(msg) = response {
                        bus.emit(PulseKind::Command, self.name(), msg);

                        if trimmed == "quit" {
                            process::exit(0);
                        }
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    bus.emit(
                        PulseKind::Command,
                        self.name(),
                        "command input channel disconnected",
                    );
                    break;
                }
            }
        }
    }
}

/// Compute an overall health score from the topology.
/// Currently: min health across all organs.
pub fn compute_overall_health(topo: &SystemTopology) -> f32 {
    if topo.organs.is_empty() {
        return 1.0;
    }
    topo.organs
        .iter()
        .map(|o| o.health)
        .fold(1.0, |acc, h| acc.min(h))
}

/// Turn a health score into a simple label.
fn classify_health(h: f32) -> &'static str {
    if h >= 0.85 {
        "ok"
    } else if h >= 0.6 {
        "degraded"
    } else if h >= 0.35 {
        "impaired"
    } else if h > 0.0 {
        "critical"
    } else {
        "failed"
    }
}

/// Very simple blocking kernel loop that runs all daemons and uses the bus.
pub fn run_loop(mut bus: Bus) {
    println!("[AION-KERNEL] Entering daemon loop. Ctrl+C to exit.");

    let topology = Arc::new(Mutex::new(organism::sample_topology()));

    // Shared metrics snapshot between status + command daemons + HTTP.
    let metrics_snapshot: Arc<Mutex<Option<TelemetrySnapshot>>> =
        Arc::new(Mutex::new(None));

    // Start tiny HTTP server (status & metrics & mem).
    let http_server = HttpServer::new("127.0.0.1:8080");
    let mem_for_http = bus.memory.clone();
    http_server.start(
        Arc::clone(&topology),
        Arc::clone(&metrics_snapshot),
        mem_for_http,
    );

    // Set up a channel + thread to read stdin commands.
    let (cmd_tx, cmd_rx) = mpsc::channel::<String>();

    thread::spawn(move || {
        let stdin = io::stdin();
        println!("[AION-CMD] Type commands: help, status, topology");
        print!("AION> ");
        io::stdout().flush().unwrap();

        for line in stdin.lock().lines() {
            match line {
                Ok(cmd) => {
                    let cmd = cmd.trim().to_string();
                    if !cmd.is_empty() {
                        let _ = cmd_tx.send(cmd);
                    }
                    print!("AION> ");
                    io::stdout().flush().unwrap();
                }
                Err(_) => break,
            }
        }
    });

    let mut daemons: Vec<Box<dyn Daemon>> = Vec::new();

    // Shared topology for all daemons.
    let topo_for_status = Arc::clone(&topology);
    let topo_for_ai = Arc::clone(&topology);
    let topo_for_sim = Arc::clone(&topology);
    let topo_for_cmd = Arc::clone(&topology);

    // Metrics snapshot clones.
    let metrics_for_status = Arc::clone(&metrics_snapshot);
    let metrics_for_cmd = Arc::clone(&metrics_snapshot);

    // Telemetry provider: select from env var AION_TELEMETRY.
    let telemetry: Box<dyn TelemetryProvider> = {
        let mode = std::env::var("AION_TELEMETRY").unwrap_or_else(|_| "sim".to_string());
        match mode.as_str() {
            "real" => {
                bus.telemetry_mode = TelemetryMode::Real;
                Box::new(RealTelemetry::new(SimLevel::Low))
            }
            _ => {
                bus.telemetry_mode = TelemetryMode::Simulated;
                Box::new(SimulatedTelemetry::new(SimLevel::Low))
            }
        }
    };

    // Later: build this list from config, discovery, etc.
    daemons.push(Box::new(HeartbeatDaemon::new(Duration::from_millis(1000))));
    daemons.push(Box::new(StatusDaemon::new(
        Duration::from_millis(5000),
        topo_for_status,
        telemetry,
        metrics_for_status,
    )));
    daemons.push(Box::new(AiDaemon::new(
        Duration::from_millis(2000),
        topo_for_ai,
    )));
    daemons.push(Box::new(SimulationDaemon::new(
        Duration::from_millis(2500),
        topo_for_sim,
    )));
    daemons.push(Box::new(CommandDaemon::new(
        cmd_rx,
        topo_for_cmd,
        metrics_for_cmd,
    )));

    loop {
        let now = Instant::now();

        for daemon in daemons.iter_mut() {
            daemon.tick(now, &mut bus);
        }

        thread::sleep(Duration::from_millis(50));
    }
}
