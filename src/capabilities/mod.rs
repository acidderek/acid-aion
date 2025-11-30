// src/capabilities/mod.rs

use std::collections::HashMap;

use crate::organism::{OrganKind, OrganId};

/// High-level capability types that AION can reason about.
///
/// These are "verbs" the AI Cortex will eventually plan with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    /// General compute / reasoning work.
    CortexCompute,

    /// Long-term storage / persistence.
    StorageIo,

    /// Fast volatile memory operations.
    MemoryAccess,

    /// Network communication.
    NetworkIo,

    /// Sensor input (camera, mic, lidar, etc.)
    SensorInput,

    /// Motor / actuator control.
    MotorControl,

    /// GPU or accelerator workloads.
    GpuWorkload,

    /// System-level orchestration (start/stop tasks, spawn nodes).
    Orchestration,

    /// Anything not yet modeled explicitly.
    Other,
}

/// A single capability instance attached to an organ.
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: u64,
    pub organ_id: OrganId,
    pub kind: CapabilityKind,
    /// Human-friendly short label.
    pub label: String,
    /// Optional free-form description.
    pub description: String,
    /// Is this capability currently usable?
    pub enabled: bool,
    /// How important this capability is for survival (0.0–1.0).
    pub priority: f32,
}

impl Capability {
    pub fn new(
        id: u64,
        organ_id: OrganId,
        kind: CapabilityKind,
        label: impl Into<String>,
        description: impl Into<String>,
        priority: f32,
    ) -> Self {
        Self {
            id,
            organ_id,
            kind,
            label: label.into(),
            description: description.into(),
            enabled: true,
            priority: priority.clamp(0.0, 1.0),
        }
    }
}

/// Convenience view when asking "what can this organ do?"
#[derive(Debug, Clone)]
pub struct OrganCapabilities {
    pub organ_id: OrganId,
    pub organ_kind: OrganKind,
    pub items: Vec<Capability>,
}

/// In-memory registry of all known capabilities in the organism.
///
/// In the future this can be backed by config / discovery.
#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    /// All capabilities by ID.
    by_id: HashMap<u64, Capability>,
    /// Capabilities grouped by organ.
    by_organ: HashMap<u32, Vec<u64>>,
    /// Simple auto-increment for IDs.
    next_id: u64,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new capability for an organ, returning its ID.
    pub fn register(
        &mut self,
        organ_id: OrganId,
        kind: CapabilityKind,
        label: impl Into<String>,
        description: impl Into<String>,
        priority: f32,
    ) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let cap = Capability::new(id, organ_id.clone(), kind, label, description, priority);
        self.by_id.insert(id, cap);

        self.by_organ
            .entry(organ_id.0)
            .or_default()
            .push(id);

        id
    }

    pub fn get(&self, id: u64) -> Option<&Capability> {
        self.by_id.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Capability> {
        self.by_id.get_mut(&id)
    }

    pub fn for_organ(&self, organ_id: OrganId) -> Vec<&Capability> {
        self.by_organ
            .get(&organ_id.0)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.by_id.get(id))
            .collect()
    }

    /// Convenience: list all capabilities that match a given kind.
    pub fn by_kind(&self, kind: CapabilityKind) -> Vec<&Capability> {
        self.by_id
            .values()
            .filter(|c| c.kind == kind)
            .collect()
    }

    /// Enable or disable a capability.
    pub fn set_enabled(&mut self, id: u64, enabled: bool) {
        if let Some(cap) = self.by_id.get_mut(&id) {
            cap.enabled = enabled;
        }
    }

    /// Simple text dump for debugging / CLI.
    pub fn describe_all(&self) -> String {
        let mut out = String::new();
        out.push_str("Capabilities:\n");
        for cap in self.by_id.values() {
            out.push_str(&format!(
                " - #{id} organ={organ} kind={kind:?} [{state}] prio={prio:.2} :: {label}\n   {desc}\n",
                id = cap.id,
                organ = cap.organ_id.0,
                kind = cap.kind,
                state = if cap.enabled { "enabled" } else { "disabled" },
                prio = cap.priority,
                label = cap.label,
                desc = cap.description,
            ));
        }
        out
    }
}
