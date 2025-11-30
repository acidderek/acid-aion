// src/memory/mod.rs

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Logical scope for a memory entry.
/// This is purely conceptual for now; later it can map to nodes/organs/tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryScope {
    Global,
    Node(u32),
    Organ(u32),
    Task(u64),
}

/// A simple typed value held in working memory.
///
/// We deliberately keep this very small and non-generic so it’s easy to
/// serialize later if needed.
#[derive(Debug, Clone)]
pub enum MemoryValue {
    Text(String),
    Number(f64),
    Flag(bool),
    /// A lightweight "bag" of key/value pairs, for small structs.
    Map(HashMap<String, MemoryValue>),
}

impl fmt::Display for MemoryValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryValue::Text(s) => write!(f, "\"{}\"", s),
            MemoryValue::Number(n) => write!(f, "{}", n),
            MemoryValue::Flag(b) => write!(f, "{}", b),
            MemoryValue::Map(m) => {
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in m {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// Internal store keyed by (scope, key).
#[derive(Debug, Default)]
struct MemoryStore {
    data: HashMap<(MemoryScope, String), MemoryValue>,
}

impl MemoryStore {
    fn set(&mut self, scope: MemoryScope, key: String, value: MemoryValue) {
        self.data.insert((scope, key), value);
    }

    fn get(&self, scope: MemoryScope, key: &str) -> Option<&MemoryValue> {
        self.data.get(&(scope, key.to_string()))
    }

    fn dump(&self) -> String {
        let mut out = String::new();
        out.push_str("Working memory snapshot:\n");
        for ((scope, key), value) in &self.data {
            out.push_str(&format!(
                " - {:?} / {} = {}\n",
                scope, key, value
            ));
        }
        out
    }
}

/// Public handle for the memory bus.
#[derive(Clone)]
pub struct MemoryBus {
    inner: Arc<Mutex<MemoryStore>>,
}

impl MemoryBus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryStore::default())),
        }
    }

    pub fn scoped(scope: MemoryScope) -> ScopedMemory {
        ScopedMemory {
            scope,
            bus: MemoryBus::new(),
        }
    }

    /// Store a text value.
    pub fn set_text(
        &self,
        scope: MemoryScope,
        key: impl Into<String>,
        value: impl Into<String>,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.set(scope, key.into(), MemoryValue::Text(value.into()));
    }

    /// Store a numeric value.
    pub fn set_number(
        &self,
        scope: MemoryScope,
        key: impl Into<String>,
        value: f64,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.set(scope, key.into(), MemoryValue::Number(value));
    }

    /// Store a boolean flag.
    pub fn set_flag(
        &self,
        scope: MemoryScope,
        key: impl Into<String>,
        value: bool,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.set(scope, key.into(), MemoryValue::Flag(value));
    }

    /// Store a small map object.
    pub fn set_map(
        &self,
        scope: MemoryScope,
        key: impl Into<String>,
        value: HashMap<String, MemoryValue>,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.set(scope, key.into(), MemoryValue::Map(value));
    }

    /// Read anything back (if present).
    pub fn get(&self, scope: MemoryScope, key: &str) -> Option<MemoryValue> {
        let guard = self.inner.lock().unwrap();
        guard.get(scope, key).cloned()
    }

    /// Produce a string dump for debugging / CLI.
    pub fn dump(&self) -> String {
        let guard = self.inner.lock().unwrap();
        guard.dump()
    }

    /// Get a cloneable, shareable handle to the inner Arc.
    pub fn inner_arc(&self) -> Arc<Mutex<MemoryStore>> {
        Arc::clone(&self.inner)
    }
}

/// A convenience wrapper that "bakes in" a scope so callers don’t need
/// to keep repeating it.
#[derive(Clone)]
pub struct ScopedMemory {
    scope: MemoryScope,
    bus: MemoryBus,
}

impl ScopedMemory {
    pub fn set_text(&self, key: impl Into<String>, value: impl Into<String>) {
        self.bus.set_text(self.scope, key, value);
    }

    pub fn set_number(&self, key: impl Into<String>, value: f64) {
        self.bus.set_number(self.scope, key, value);
    }

    pub fn set_flag(&self, key: impl Into<String>, value: bool) {
        self.bus.set_flag(self.scope, key, value);
    }

    pub fn get(&self, key: &str) -> Option<MemoryValue> {
        self.bus.get(self.scope, key)
    }

    pub fn dump(&self) -> String {
        self.bus.dump()
    }

    /// Expose underlying MemoryBus if needed for sharing with other scopes.
    pub fn bus(&self) -> MemoryBus {
        self.bus.clone()
    }

    pub fn scope(&self) -> MemoryScope {
        self.scope
    }
}
