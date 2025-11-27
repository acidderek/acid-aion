// src/kernel/mod.rs

use std::fs;
use std::io::{self, BufRead, Write};
use std::process;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::organism::{
    self, compute_awareness, format_topology_brief, OrganKind, SystemTopology,
};

/// Different categories of pulses travelling on the bus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PulseKind {
    Heartbeat,
    Status,
    Command,
    Ai,
}

/// Very simple message bus for now: just logs pulses with an incrementing ID.
/// Later this can route to organs, nodes, external transports, etc.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogFilter {
    All,
    CommandsOnly,
    Silent,
}

pub struct Bus {
    next_id: u64,
    pub log_filter: LogFilter,
    pub awareness_score: f32, // 0.0–1.0
}

impl Bus {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            log_filter: LogFilter::CommandsOnly, // default: clean UI
            awareness_score: 1.0,                // start fully aware
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
pub struct StatusDaemon {
    last_run: Instant,
    interval: Duration,
    counter: u64,
    topology: SystemTopology,
}

impl StatusDaemon {
    pub fn new(interval: Duration, topology: SystemTopology) -> Self {
        Self {
            last_run: Instant::now(),
            interval,
            counter: 0,
            topology,
        }
    }
}

impl Daemon for StatusDaemon {
    fn name(&self) -> &'static str {
        "status"
    }

    fn tick(&mut self, now: Instant, bus: &mut Bus) {
        if now.duration_since(self.last_run) >= self.interval {
            self.counter += 1;
            self.last_run = now;

            let brief = format_topology_brief(&self.topology);

            // Compute min organ health as a crude "system health" metric
            let mut min_health = 1.0f32;
            for organ in &self.topology.organs {
                if organ.health < min_health {
                    min_health = organ.health;
                }
            }

            let health_level = if min_health >= 0.85 {
                "healthy"
            } else if min_health >= 0.6 {
                "degraded"
            } else if min_health >= 0.35 {
                "impaired"
            } else if min_health > 0.0 {
                "critical"
            } else {
                "failed"
            };

            let awareness = compute_awareness(&self.topology);
            bus.awareness_score = awareness;

            bus.emit(
                PulseKind::Status,
                self.name(),
                format!(
                    "status tick #{} :: {} :: health {:.2} ({}) :: awareness {:.2}",
                    self.counter, brief, min_health, health_level, awareness
                ),
            );
        }
    }
}

/// A daemon representing the AI Cortex: all high-level intelligence lives here.
/// It reacts to the awareness index carried on the bus.
pub struct AiDaemon {
    last_run: Instant,
    interval: Duration,
    cycle: u64,
}

impl AiDaemon {
    pub fn new(interval: Duration) -> Self {
        Self {
            last_run: Instant::now(),
            interval,
            cycle: 0,
        }
    }
}

impl Daemon for AiDaemon {
    fn name(&self) -> &'static str {
        "ai-cortex"
    }

    fn tick(&mut self, now: Instant, bus: &mut Bus) {
        if now.duration_since(self.last_run) >= self.interval {
            self.cycle += 1;
            self.last_run = now;

            let awareness = bus.awareness_score;
            let level = if awareness >= 0.85 {
                "optimal"
            } else if awareness >= 0.6 {
                "stable"
            } else if awareness >= 0.35 {
                "impaired"
            } else if awareness > 0.0 {
                "critical"
            } else {
                "unconscious"
            };

            bus.emit(
                PulseKind::Ai,
                self.name(),
                format!(
                    "cortex cycle #{} :: awareness {:.2} ({})",
                    self.cycle, awareness, level
                ),
            );
        }
    }
}

/// A daemon that processes user commands from stdin.
/// This is the first AION "shell" interface.
pub struct CommandDaemon {
    rx: Receiver<String>,
    topology: SystemTopology,
}

impl CommandDaemon {
    pub fn new(rx: Receiver<String>, topology: SystemTopology) -> Self {
        Self { rx, topology }
    }

    fn parse_organ_kind(&self, name: &str) -> Option<OrganKind> {
        match name.to_lowercase().as_str() {
            "cortex" => Some(OrganKind::Cortex),
            "memory" => Some(OrganKind::Memory),
            "io" | "io_bridge" | "io-bridge" | "iobridge" => Some(OrganKind::IoBridge),
            _ => None,
        }
    }

    fn adjust_organ_health(&mut self, kind: OrganKind, delta: f32) -> Option<f32> {
        for organ in self.topology.organs.iter_mut() {
            if organ.kind == kind {
                let new_health = (organ.health + delta).clamp(0.0, 1.0);
                organ.health = new_health;
                return Some(new_health);
            }
        }
        None
    }

    fn classify_health_label(&self, health: f32) -> &'static str {
        if health >= 0.85 {
            "ok"
        } else if health >= 0.6 {
            "degraded"
        } else if health >= 0.35 {
            "impaired"
        } else if health > 0.0 {
            "critical"
        } else {
            "failed"
        }
    }

    fn health_severity(&self, health: f32) -> u8 {
        if health >= 0.85 {
            0 // ok
        } else if health >= 0.6 {
            1 // degraded
        } else if health >= 0.35 {
            2 // impaired
        } else if health > 0.0 {
            3 // critical
        } else {
            4 // failed
        }
    }

    /// Save organ health to a simple text file.
    /// Format (one per line): <OrganKindName> <health>
    fn save_state(&self, path: &str) -> Result<(), String> {
        let mut out = String::new();
        for organ in &self.topology.organs {
            let name = match organ.kind {
                OrganKind::Cortex => "Cortex",
                OrganKind::Memory => "Memory",
                OrganKind::IoBridge => "IoBridge",
                OrganKind::SensorHub => "SensorHub",
                OrganKind::MotorControl => "MotorControl",
                OrganKind::Network => "Network",
                OrganKind::Storage => "Storage",
            };
            out.push_str(&format!("{} {:.4}\n", name, organ.health));
        }

        fs::write(path, out).map_err(|e| format!("failed to save state: {}", e))?;
        Ok(())
    }

    /// Load organ health from a simple text file.
    /// Unknown organs or malformed lines are ignored.
    fn load_state(&mut self, path: &str) -> Result<(), String> {
        let data = fs::read_to_string(path).map_err(|e| format!("failed to load state: {}", e))?;

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 2 {
                continue;
            }

            let organ_name = parts[0];
            let value_str = parts[1];

            // First: try the generic parser (handles cortex/memory/io/etc.)
            let organ_kind_opt = self
                .parse_organ_kind(organ_name)
                .or_else(|| match organ_name {
                    // Also support exact names used when saving
                    "IoBridge" => Some(OrganKind::IoBridge),
                    "SensorHub" => Some(OrganKind::SensorHub),
                    "MotorControl" => Some(OrganKind::MotorControl),
                    "Network" => Some(OrganKind::Network),
                    "Storage" => Some(OrganKind::Storage),
                    _ => None,
                });

            let organ_kind = match organ_kind_opt {
                Some(k) => k,
                None => continue,
            };

            let value: f32 = match value_str.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Apply new health
            for organ in self.topology.organs.iter_mut() {
                if organ.kind == organ_kind {
                    organ.health = value.clamp(0.0, 1.0);
                }
            }
        }

        Ok(())
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

                    // Parameterized commands: damage/heal
                    if let Some(rest) = trimmed.strip_prefix("damage ") {
                        let parts: Vec<&str> = rest.split_whitespace().collect();
                        if parts.len() != 2 {
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                "usage: damage <organ> <amount>",
                            );
                            continue;
                        }

                        let organ_name = parts[0];
                        let amount_str = parts[1];

                        let organ_kind = match self.parse_organ_kind(organ_name) {
                            Some(k) => k,
                            None => {
                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!("unknown organ '{}'", organ_name),
                                );
                                continue;
                            }
                        };

                        let amount: f32 = match amount_str.parse() {
                            Ok(v) => v,
                            Err(_) => {
                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!("invalid amount '{}'", amount_str),
                                );
                                continue;
                            }
                        };

                        let delta = -amount.abs();
                        match self.adjust_organ_health(organ_kind, delta) {
                            Some(new_health) => {
                                let awareness = compute_awareness(&self.topology);
                                bus.awareness_score = awareness;

                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!(
                                        "damaged {:?} by {:.2}, new health {:.2} (awareness {:.2})",
                                        organ_kind, amount, new_health, awareness
                                    ),
                                );
                            }
                            None => {
                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!(
                                        "organ {:?} not found in topology",
                                        organ_kind
                                    ),
                                );
                            }
                        }

                        continue;
                    }

                    if let Some(rest) = trimmed.strip_prefix("heal ") {
                        let parts: Vec<&str> = rest.split_whitespace().collect();
                        if parts.len() != 2 {
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                "usage: heal <organ> <amount>",
                            );
                            continue;
                        }

                        let organ_name = parts[0];
                        let amount_str = parts[1];

                        let organ_kind = match self.parse_organ_kind(organ_name) {
                            Some(k) => k,
                            None => {
                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!("unknown organ '{}'", organ_name),
                                );
                                continue;
                            }
                        };

                        let amount: f32 = match amount_str.parse() {
                            Ok(v) => v,
                            Err(_) => {
                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!("invalid amount '{}'", amount_str),
                                );
                                continue;
                            }
                        };

                        let delta = amount.abs();
                        match self.adjust_organ_health(organ_kind, delta) {
                            Some(new_health) => {
                                let awareness = compute_awareness(&self.topology);
                                bus.awareness_score = awareness;

                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!(
                                        "healed {:?} by {:.2}, new health {:.2} (awareness {:.2})",
                                        organ_kind, amount, new_health, awareness
                                    ),
                                );
                            }
                            None => {
                                bus.emit(
                                    PulseKind::Command,
                                    self.name(),
                                    format!(
                                        "organ {:?} not found in topology",
                                        organ_kind
                                    ),
                                );
                            }
                        }

                        continue;
                    }

                    // Simple commands (no parameters)
                    match trimmed {
                        "help" => {
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                "commands: help, status, topology, nodes, organs, peripherals, health, awareness, alerts, save state, load state, damage <organ> <amount>, heal <organ> <amount>, logs all, logs commands, logs silent, quit",
                            );
                        }

                        "status" => {
                            let brief = format_topology_brief(&self.topology);
                            let awareness = compute_awareness(&self.topology);
                            bus.awareness_score = awareness;

                            // Also compute overall system health for human-readable status
                            let mut min_health = 1.0f32;
                            for organ in &self.topology.organs {
                                if organ.health < min_health {
                                    min_health = organ.health;
                                }
                            }
                            let health_level = self.classify_health_label(min_health);

                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                format!(
                                    "manual status :: {} :: health {:.2} ({}) :: awareness {:.2}",
                                    brief, min_health, health_level, awareness
                                ),
                            );
                        }

                        "topology" => {
                            let mut details = String::new();
                            details.push_str("Topology detail:\n");
                            for node in &self.topology.nodes {
                                details.push_str(&format!(
                                    " - Node {} [{}]: {}\n",
                                    node.id.0, node.label, node.role
                                ));
                            }
                            for organ in &self.topology.organs {
                                details.push_str(&format!(
                                    "   - Organ {:?} on Node {} (health {:.2})\n",
                                    organ.kind, organ.node.0, organ.health
                                ));
                            }

                            bus.emit(PulseKind::Command, self.name(), details);
                        }

                        "nodes" => {
                            let mut details = String::new();
                            details.push_str("Nodes:\n");
                            for node in &self.topology.nodes {
                                details.push_str(&format!(
                                    " - Node {} [{}]: {}\n",
                                    node.id.0, node.label, node.role
                                ));
                            }
                            bus.emit(PulseKind::Command, self.name(), details);
                        }

                        "organs" => {
                            let mut details = String::new();
                            details.push_str("Organs:\n");
                            for organ in &self.topology.organs {
                                details.push_str(&format!(
                                    " - Organ {:?} on Node {} (health {:.2})\n",
                                    organ.kind, organ.node.0, organ.health
                                ));
                            }
                            bus.emit(PulseKind::Command, self.name(), details);
                        }

                        "peripherals" => {
                            let mut details = String::new();
                            details.push_str("Peripherals by organ:\n");
                            for organ in &self.topology.organs {
                                if organ.peripherals.is_empty() {
                                    continue;
                                }
                                details.push_str(&format!(" - Organ {:?}:\n", organ.kind));
                                for p in &organ.peripherals {
                                    details.push_str(&format!(
                                        "    - {:?}: {}\n",
                                        p.kind, p.name
                                    ));
                                }
                            }
                            if !details.contains("Organ") {
                                details.push_str(" (no peripherals registered)\n");
                            }
                            bus.emit(PulseKind::Command, self.name(), details);
                        }

                        "health" => {
                            let mut details = String::new();
                            details.push_str("Organ health:\n");
                            for organ in &self.topology.organs {
                                let label = self.classify_health_label(organ.health);
                                details.push_str(&format!(
                                    " - {:?}: {:.2} ({})\n",
                                    organ.kind, organ.health, label
                                ));
                            }
                            bus.emit(PulseKind::Command, self.name(), details);
                        }

                        "alerts" => {
                            let mut details = String::new();
                            let mut worst_severity = 0u8;
                            let mut any_alerts = false;

                            details.push_str("Alerts:\n");

                            for organ in &self.topology.organs {
                                let sev = self.health_severity(organ.health);
                                if sev == 0 {
                                    continue; // ok, no alert
                                }
                                any_alerts = true;
                                if sev > worst_severity {
                                    worst_severity = sev;
                                }

                                let label = self.classify_health_label(organ.health);
                                details.push_str(&format!(
                                    " - {:?}: {:.2} [{}]\n",
                                    organ.kind, organ.health, label
                                ));
                            }

                            if !any_alerts {
                                details.push_str(" (no active alerts; all organs healthy)\n");
                            } else {
                                let overall = match worst_severity {
                                    1 => "overall: degraded",
                                    2 => "overall: impaired",
                                    3 => "overall: critical",
                                    4 => "overall: failed",
                                    _ => "overall: ok",
                                };
                                details.push_str(overall);
                                details.push('\n');
                            }

                            bus.emit(PulseKind::Command, self.name(), details);
                        }

                        "awareness" => {
                            let score = compute_awareness(&self.topology);
                            bus.awareness_score = score;

                            let level = if score >= 0.85 {
                                "optimal"
                            } else if score >= 0.6 {
                                "stable"
                            } else if score >= 0.35 {
                                "impaired"
                            } else if score > 0.0 {
                                "critical"
                            } else {
                                "unconscious"
                            };

                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                format!("awareness index: {:.2} :: {}", score, level),
                            );
                        }

                        "save state" => {
                            match self.save_state("aion_state.txt") {
                                Ok(()) => {
                                    bus.emit(
                                        PulseKind::Command,
                                        self.name(),
                                        "state saved to aion_state.txt",
                                    );
                                }
                                Err(e) => {
                                    bus.emit(
                                        PulseKind::Command,
                                        self.name(),
                                        format!("save failed: {}", e),
                                    );
                                }
                            }
                        }

                        "load state" => {
                            match self.load_state("aion_state.txt") {
                                Ok(()) => {
                                    let awareness = compute_awareness(&self.topology);
                                    bus.awareness_score = awareness;

                                    bus.emit(
                                        PulseKind::Command,
                                        self.name(),
                                        format!(
                                            "state loaded from aion_state.txt (awareness {:.2})",
                                            awareness
                                        ),
                                    );
                                }
                                Err(e) => {
                                    bus.emit(
                                        PulseKind::Command,
                                        self.name(),
                                        format!("load failed: {}", e),
                                    );
                                }
                            }
                        }

                        "logs all" => {
                            bus.log_filter = LogFilter::All;
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                "logging: ALL pulses",
                            );
                        }

                        "logs commands" => {
                            bus.log_filter = LogFilter::CommandsOnly;
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                "logging: COMMANDS ONLY",
                            );
                        }

                        "logs silent" | "logs off" => {
                            bus.log_filter = LogFilter::Silent;
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                "logging: SILENT",
                            );
                        }

                        "quit" => {
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                "shutting down kernel (process::exit(0))",
                            );
                            process::exit(0);
                        }

                        _ => {
                            bus.emit(
                                PulseKind::Command,
                                self.name(),
                                format!("unknown command: '{}'", trimmed),
                            );
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

/// Very simple blocking kernel loop that runs all daemons and uses the bus.
pub fn run_loop(mut bus: Bus) {
    println!("[AION-KERNEL] Entering daemon loop. Ctrl+C to exit.");

    let topology = organism::sample_topology();

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

    daemons.push(Box::new(HeartbeatDaemon::new(Duration::from_millis(1000))));
    daemons.push(Box::new(StatusDaemon::new(
        Duration::from_millis(5000),
        topology.clone(),
    )));
    daemons.push(Box::new(AiDaemon::new(Duration::from_millis(2000))));
    daemons.push(Box::new(CommandDaemon::new(cmd_rx, topology)));

    loop {
        let now = Instant::now();

        for daemon in daemons.iter_mut() {
            daemon.tick(now, &mut bus);
        }

        thread::sleep(Duration::from_millis(50));
    }
}
