//! Organism model for AION.
//!
//! Represents the system as nodes + organs + peripherals,
//! with health and awareness semantics.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganKind {
    Cortex,
    Memory,
    IoBridge,
    SensorHub,
    MotorControl,
    Network,
    Storage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    Compute,
    Perception,
    Actuation,
    Storage,
    Networking,
    Planning,
    Learning,
}

#[derive(Debug, Clone, Copy)]
pub enum PeripheralKind {
    Cpu,
    Gpu,
    Nic,
    Disk,
    Usb,
    Sensor,
    Motor,
    Display,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Peripheral {
    pub kind: PeripheralKind,
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct OrganId(pub u32);

#[derive(Debug, Clone)]
pub struct Organ {
    pub id: OrganId,
    pub node: NodeId,
    pub kind: OrganKind,
    pub caps: Vec<CapabilityKind>,
    pub health: f32, // 0.0–1.0
    pub peripherals: Vec<Peripheral>,
}

impl Organ {
    /// Does this organ provide a specific capability?
    pub fn has_capability(&self, cap: CapabilityKind) -> bool {
        self.caps.iter().any(|c| *c == cap)
    }

    /// Does this organ provide any of the listed capabilities?
    pub fn has_any_capability(&self, caps: &[CapabilityKind]) -> bool {
        self.caps.iter().any(|c| caps.contains(c))
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct SystemTopology {
    pub nodes: Vec<Node>,
    pub organs: Vec<Organ>,
}

/// Build a simple sample topology:
///
/// - Node 1: core-0 (primary brain)
///   - Cortex organ
///   - Memory organ
/// - Node 2: io-0 (peripheral bridge)
///   - IoBridge organ
pub fn sample_topology() -> SystemTopology {
    let node_core = Node {
        id: NodeId(1),
        label: "core-0".to_string(),
        role: "primary brain".to_string(),
    };

    let node_io = Node {
        id: NodeId(2),
        label: "io-0".to_string(),
        role: "peripheral bridge".to_string(),
    };

    let cortex = Organ {
        id: OrganId(1),
        node: node_core.id,
        kind: OrganKind::Cortex,
        caps: vec![
            CapabilityKind::Compute,
            CapabilityKind::Planning,
            CapabilityKind::Learning,
        ],
        health: 0.98,
        peripherals: vec![
            Peripheral {
                kind: PeripheralKind::Cpu,
                name: "Sim-CPU-0".to_string(),
            },
            Peripheral {
                kind: PeripheralKind::Gpu,
                name: "Sim-GPU-0".to_string(),
            },
        ],
    };

    let memory = Organ {
        id: OrganId(2),
        node: node_core.id,
        kind: OrganKind::Memory,
        caps: vec![
            CapabilityKind::Storage,
            CapabilityKind::Perception,
        ],
        health: 0.99,
        peripherals: vec![Peripheral {
            kind: PeripheralKind::Disk,
            name: "Sim-NVMe-0".to_string(),
        }],
    };

    let io_bridge = Organ {
        id: OrganId(3),
        node: node_io.id,
        kind: OrganKind::IoBridge,
        caps: vec![
            CapabilityKind::Networking,
            CapabilityKind::Actuation,
        ],
        health: 0.97,
        peripherals: vec![
            Peripheral {
                kind: PeripheralKind::Nic,
                name: "Sim-10G-NIC-0".to_string(),
            },
            Peripheral {
                kind: PeripheralKind::Usb,
                name: "Sim-USB-Hub-0".to_string(),
            },
            Peripheral {
                kind: PeripheralKind::Display,
                name: "Sim-Display-0".to_string(),
            },
        ],
    };

    SystemTopology {
        nodes: vec![node_core, node_io],
        organs: vec![cortex, memory, io_bridge],
    }
}

/// Return a brief summary used in status messages.
pub fn format_topology_brief(topology: &SystemTopology) -> String {
    let node_count = topology.nodes.len();
    let organ_count = topology.organs.len();

    let mut node_labels = Vec::new();
    for node in &topology.nodes {
        node_labels.push(format!("{} ({})", node.label, node.role));
    }

    if node_labels.is_empty() {
        format!("{} node(s), {} organ(s)", node_count, organ_count)
    } else {
        format!(
            "{} node(s), {} organ(s) :: {}",
            node_count,
            organ_count,
            node_labels.join(", ")
        )
    }
}

/// Compute an awareness index (0.0–1.0) from organ healths.
///
/// Phase 1: weighted by core organs.
/// - Cortex  : 0.4
/// - Memory  : 0.3
/// - IoBridge: 0.3
pub fn compute_awareness(topology: &SystemTopology) -> f32 {
    let mut cortex_h = 1.0;
    let mut memory_h = 1.0;
    let mut io_h = 1.0;

    for organ in &topology.organs {
        match organ.kind {
            OrganKind::Cortex => cortex_h = organ.health,
            OrganKind::Memory => memory_h = organ.health,
            OrganKind::IoBridge => io_h = organ.health,
            _ => {}
        }
    }

    let awareness = 0.4 * cortex_h + 0.3 * memory_h + 0.3 * io_h;
    awareness.clamp(0.0, 1.0)
}

/// Turn an awareness score into a human-readable label.
///
/// - ≥ 0.85       → "optimal"
/// - 0.60 – 0.85  → "stable"
/// - 0.35 – 0.60  → "impaired"
/// - 0.01 – 0.35  → "critical"
/// - 0.0          → "unconscious"
pub fn describe_awareness(a: f32) -> &'static str {
    let v = a.clamp(0.0, 1.0);
    if v >= 0.85 {
        "optimal"
    } else if v >= 0.60 {
        "stable"
    } else if v >= 0.35 {
        "impaired"
    } else if v > 0.0 {
        "critical"
    } else {
        "unconscious"
    }
}

/// Find all organs that provide a given capability.
pub fn organs_with_capability<'a>(
    topology: &'a SystemTopology,
    cap: CapabilityKind,
) -> Vec<&'a Organ> {
    topology
        .organs
        .iter()
        .filter(|o| o.has_capability(cap))
        .collect()
}

/// Find all organs that provide *any* of the requested capabilities.
pub fn organs_with_any_capability<'a>(
    topology: &'a SystemTopology,
    caps: &[CapabilityKind],
) -> Vec<&'a Organ> {
    topology
        .organs
        .iter()
        .filter(|o| o.has_any_capability(caps))
        .collect()
}
