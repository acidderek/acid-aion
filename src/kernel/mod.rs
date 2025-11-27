use std::io::{self, BufRead, Write};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};
use std::process; 

use crate::organism::{self, SystemTopology, format_topology_brief};

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
}

impl Bus {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            log_filter: LogFilter::CommandsOnly, // default: clean UI
        }
    }

    pub fn emit(&mut self, kind: PulseKind, source: &'static str, data: impl Into<String>) {
        self.next_id += 1;
        let data = data.into();

        // LOG RULES:
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

            bus.emit(
                PulseKind::Status,
                self.name(),
                format!("status tick #{} :: {}", self.counter, brief),
            );
        }
    }
}

/// A daemon representing the AI Cortex: all high-level intelligence lives here.
///
/// In this early phase it just emits simulated "cortex cycles".
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

            // Later: this is where model inference, planning, and learning would run.
            bus.emit(
                PulseKind::Ai,
                self.name(),
                format!("cortex cycle #{} (simulated)", self.cycle),
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

 match trimmed {
    "help" => {
        bus.emit(
            PulseKind::Command,
            self.name(),
            "commands: help, status, topology, nodes, organs, peripherals, logs all, logs commands, logs silent, quit",
        );
    }

    "status" => {
        let brief = format_topology_brief(&self.topology);
        bus.emit(
            PulseKind::Command,
            self.name(),
            format!("manual status :: {}", brief),
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
                details.push_str(&format!("    - {:?}: {}\n", p.kind, p.name));
            }
        }
        if !details.contains("Organ") {
            details.push_str(" (no peripherals registered)\n");
        }
        bus.emit(PulseKind::Command, self.name(), details);
    }

    "logs all" => {
        bus.log_filter = LogFilter::All;
        bus.emit(PulseKind::Command, self.name(), "logging: ALL pulses");
    }

    "logs commands" => {
        bus.log_filter = LogFilter::CommandsOnly;
        bus.emit(PulseKind::Command, self.name(), "logging: COMMANDS ONLY");
    }

    "logs silent" | "logs off" => {
        bus.log_filter = LogFilter::Silent;
        bus.emit(PulseKind::Command, self.name(), "logging: SILENT");
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

    // Later: build this list from config, discovery, etc.
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

        // crude scheduler: sleep a bit to avoid busy loop
        thread::sleep(Duration::from_millis(50));
    }
}
