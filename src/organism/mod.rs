// src/organism/mod.rs

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrganKind {
    Cortex,
    Memory,
    IoBridge,
    SensorHub,
    MotorControl,
    Network,
    Storage,
}

#[derive(Debug, Clone, Copy)]
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

/// A fixed sample topology for the AION sim-kernel.
/// core-0: primary brain
/// io-0:   peripheral bridge
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
        caps: vec![CapabilityKind::Storage],
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
            CapabilityKind::Perception,
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

/// Compute an overall awareness index for the system based on organ health.
/// 0.0 = unconscious / failed, 1.0 = fully healthy.
/// Cortex, Memory, and IoBridge are weighted as primary contributors.
pub fn compute_awareness(topology: &SystemTopology) -> f32 {
    let mut cortex_health = 1.0f32;
    let mut memory_health = 1.0f32;
    let mut io_health = 1.0f32;

    for organ in &topology.organs {
        match organ.kind {
            OrganKind::Cortex => cortex_health = organ.health,
            OrganKind::Memory => memory_health = organ.health,
            OrganKind::IoBridge => io_health = organ.health,
            _ => {}
        }
    }

    let score = cortex_health * 0.5 + memory_health * 0.3 + io_health * 0.2;
    score.clamp(0.0, 1.0)
}

/// Compact summary for status daemon / commands.
pub fn format_topology_brief(topology: &SystemTopology) -> String {
    let node_count = topology.nodes.len();
    let organ_count = topology.organs.len();

    let mut roles: Vec<String> = Vec::new();
    for node in &topology.nodes {
        roles.push(format!("{} ({})", node.label, node.role));
    }

    format!(
        "{} node(s), {} organ(s) :: {}",
        node_count,
        organ_count,
        roles.join(", ")
    )
}
