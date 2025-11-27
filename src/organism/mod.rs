#![allow(dead_code)] // We are intentionally defining more than we use (for now).

#[derive(Debug, Clone, Copy)]
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

impl SystemTopology {
    pub fn find_organs_on_node(&self, node_id: NodeId) -> Vec<&Organ> {
        self.organs
            .iter()
            .filter(|o| o.node.0 == node_id.0)
            .collect()
    }
}

/// A small hard-coded topology for the early simulation.
///
/// Later this will be discovered from real hardware and config.
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
            CapabilityKind::Perception,
            CapabilityKind::Actuation,
            CapabilityKind::Networking,
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

/// Compact human-readable summary for status reports.
pub fn format_topology_brief(topology: &SystemTopology) -> String {
    let node_count = topology.nodes.len();
    let organ_count = topology.organs.len();

    let node_labels: Vec<String> = topology
        .nodes
        .iter()
        .map(|n| format!("{} ({})", n.label, n.role))
        .collect();

    format!(
        "{} node(s), {} organ(s) :: {}",
        node_count,
        organ_count,
        node_labels.join(", ")
    )
}
