1. Purpose
2. Current Organism Model
3. Conceptual Layering
4. Organ Mapping to Real Subsystems
5. Health, Awareness & Alerts Mapping
  5.1 Organ Health
  5.2 Awareness Index
  5.3 Alerts
6. Telemetry vs Simulation
7. Real-World Flows
8. Phase 1 Implementation Steps
9. Long-Term Vision Hook
##



# AION Phase 1 Mapping  
**From Simulated Organism → Real System**

## 1. Purpose

This document explains how the current **AION organism simulation** (organs, health, awareness, sim events) maps onto a **real system** in Phase 1.

Phase 1 is *not yet* a full OS kernel. Instead, AION acts as an **organism-style supervisor** that:

- Maintains an internal “body model” of the machine.
- Tracks health of critical subsystems (organs).
- Computes an **awareness index** (how “ready” the system is).
- Surfaces alerts and state through the AION shell and bus.
- Is designed so that later we can:
  - plug in *real telemetry* instead of simulated events
  - insert *real AI* into the Cortex

This document is the bridge between:

- what we have now (Rust sim)
- and the **AI-native OS** we want to build later.


## 2. Current Organism Model: Quick Recap

### 2.1 Core Concepts

- **Organism**
  - Represents the overall system as a living body.
- **Nodes**
  - Logical machines / major locations in the organism.
  - In Phase 1, this is usually just:
    - `core-0` (primary brain node)
    - `io-0` (I/O node / peripheral bridge)
- **Organs**
  - High-level functional blocks inside the organism.
- **Peripherals**
  - Concrete devices attached to organs (CPU, GPU, disks, NICs, etc).

### 2.2 Organs

Right now, we model three primary organs:

- `Cortex`
  - “Thinking” / compute / planning.
- `Memory`
  - Storage, persistence, and state recall.
- `IoBridge`
  - I/O, networking, external devices.

Each organ has:

- `health: f32 (0.0..=1.0)`
- `caps: Vec<CapabilityKind>`
- `peripherals: Vec<Peripheral>`

### 2.3 Health & Awareness

- **Organ health**: represents the local condition of that subsystem.
- **Awareness index**: a derived score `0.0..=1.0` computed from organ healths.
  - Rough interpretation:
    - `≥ 0.85` — **optimal**
    - `0.60–0.85` — **stable**
    - `0.35–0.60` — **impaired**
    - `0.01–0.35` — **critical**
    - `0.0` — **unconscious**

### 2.4 Daemons & Bus

- **Bus**
  - Carries pulses (`PulseKind`) between daemons and logs.
  - Tracks global state: `log_filter`, `awareness_score`, `sim_level`.
- **Daemons**
  - `HeartbeatDaemon`: basic clock.
  - `StatusDaemon`: periodic status, health, awareness.
  - `AiDaemon`: Cortex placeholder reacting to awareness.
  - `SimulationDaemon`: fake world events (overheat, pressure, congestion, recovery).
  - `CommandDaemon`: AION shell (commands like `health`, `alerts`, `sim status`, `damage`, `heal`, `save state`, `load state`, etc).

- **Persistence**
  - `save state` / `load state` write/read `aion_state.txt` with organ health values.


## 3. Conceptual Layering (Phase 1)

Phase 1 **does not** require AION to boot as a real OS kernel. Instead, think of it as:

1. **Physical Machine**
   - CPU, RAM, GPU(s), disks, NICs, USB, sensors, etc.

2. **Host Runtime (for now)**
   - Could be a conventional OS or a minimal runtime.
   - Provides:
     - process execution
     - basic IO / file system / networking
   - AION runs as a **process** in this world.

3. **AION Organism Layer (current project)**
   - Our Rust code (`kernel`, `organism`, `runtime`).
   - Maintains an internal “body map” of the machine:
     - organs
     - health
     - awareness
     - state persistence

4. **AI & Policy Layer (future phases)**
   - Real AI models / policies.
   - Decides how to:
     - throttle / prioritize workloads
     - adjust simulation/intensity
     - schedule maintenance, backup, migration, etc.

5. **Human Interface**
   - AION shell (the CLI we currently have).
   - Future:
     - GUI dashboard
     - remote control / API
     - integration into dev tools, VS Code, etc.


## 4. Organ Mapping to Real Subsystems

### 4.1 Cortex

**Sim meaning now:**

- Abstract “thinking” organ.
- Health influenced by:
  - simulated CPU overheat
  - manual `damage cortex X` / `heal cortex X`
- Awareness uses Cortex health as a significant component.

**Real mapping (Phase 1–2):**

- **Subsystems:**
  - CPU cores, CPU temperature, CPU throttling.
  - GPU(s) used for ML/compute.
  - Scheduler / orchestrator health (if running multiple workloads).

- **Example metrics:**
  - CPU usage (% per core, moving average).
  - CPU temperature vs thermal limits.
  - Throttling events.
  - GPU utilization & memory usage.
  - ML job queue length / latency.

- **How to derive health:**
  - Start with `health = 1.0`.
  - Penalize when:
    - sustained high temp near throttle point.
    - frequent thermal throttling.
    - GPU memory pressure / constant OOM.
    - scheduler backlog too high for too long.
  - Penalties are small and gradual, recovery is also gradual.

### 4.2 Memory

**Sim meaning now:**

- Abstract storage/persistence organ.
- Health influenced by:
  - simulated memory pressure
  - manual `damage memory X` / `heal memory X`.

**Real mapping (Phase 1–2):**

- **Subsystems:**
  - RAM usage and pressure.
  - Swap / paging behavior.
  - Disk I/O queues & latency (for persistence).
  - Cache hit rates (optional).

- **Example metrics:**
  - Percent RAM used vs total.
  - Page faults per second.
  - Swap in/out rate.
  - Disk latency / IOPS vs “healthy” baseline.
  - Filesystem error counts.

- **How to derive health:**
  - Start at `1.0`.
  - Gradually reduce when:
    - RAM consistently above certain thresholds (e.g. 80%+ sustained).
    - heavy swapping/paging indicating memory pressure.
    - storage latency spikes beyond baseline for extended periods.
    - recurring IO errors / filesystem issues.
  - Gradually increase when:
    - pressure decreases, errors cease, IO metrics return to baseline.

### 4.3 IoBridge

**Sim meaning now:**

- Represents the connections to the outside world.
- Health influenced by:
  - simulated “IO congestion” events
  - manual `damage io ...` (through `IoBridge` organ) / `heal`.

**Real mapping (Phase 1–2):**

- **Subsystems:**
  - Network stack (NIC, routing, connectivity).
  - USB / peripheral buses.
  - Storage buses (SATA/NVMe/PCIe).
  - Anything that is “bridge between AION and environment”.

- **Example metrics:**
  - Network packet loss / error rates / latency.
  - NIC link status and speed.
  - Queue lengths / drops in network or IO.
  - PCIe error counters (if available).
  - Repeated disconnects on USB or other peripheral buses.

- **How to derive health:**
  - Start at `1.0`.
  - Reduce when:
    - persistent packet loss or high latency.
    - repeated link flaps / NIC resets.
    - IO queues consistently saturated or dropping requests.
  - Increase when:
    - connectivity stabilizes and queue lengths return to normal.
    - error counters stop incrementing.


## 5. Health, Awareness & Alerts Mapping

### 5.1 Organ Health (0.0–1.0)

Semantics:

- `1.0` — perfect health / no known issues.
- `~0.8–0.9` — small risk factors but within comfort.
- `0.6–0.8` — degraded but operating.
- `0.35–0.6` — impaired; requires attention.
- `0.01–0.35` — critical; likely impacting workloads.
- `0.0` — failed / unavailable.

**Implementation idea (Phase 1):**

- For each organ, define a function:

  ```rust
  fn compute_cortex_health(metrics: &CpuGpuMetrics) -> f32 { ... }
  fn compute_memory_health(metrics: &MemoryStorageMetrics) -> f32 { ... }
  fn compute_iobridge_health(metrics: &IoNetworkMetrics) -> f32 { ... }
These functions translate raw measurements into a normalized 0.0–1.0 value.

5.2 Awareness Index

Awareness is a composite of organ healths.

The current sim uses a simple function in compute_awareness(&SystemTopology):

Typically:

Either min(healths),

or some weighted average.

Phase 1 mapping:

Keep awareness as “how ready AION is to act intelligently”.

Use a weighted scheme:

Cortex: 0.4

Memory: 0.3

IoBridge: 0.3

So that critical compute issues strongly impact awareness, but bad IO also matters.

5.3 Alerts

alerts in the CLI correspond to health thresholds:

degraded — warning; keep an eye on it.

impaired — likely impacting response times / reliability.

critical — high risk of failure; mitigation ASAP.

failed — subsystem effectively down.

In a real system, alerts triggered by health transitions should be:

logged

possibly persisted (for later analysis)

optionally fed into AI policy / maintenance scheduling

6. Telemetry vs Simulation
6.1 SimulationDaemon Today

Right now, SimulationDaemon does:

Every few seconds:

introduces damage to Cortex / Memory / IoBridge

or applies recovery to all organs

Emits [BUS][Sim] pulses like:

cpu_overheat_mild

memory_pressure

io_congestion

recovery

This is a stand-in for real-world events.

6.2 Phase 1 Telemetry Abstraction

To move toward real metrics, we introduce a telemetry layer:

// Pseudo-interface (not implemented yet)
pub trait TelemetryProvider {
    fn read_cpu_gpu_metrics(&self) -> CpuGpuMetrics;
    fn read_memory_metrics(&self) -> MemoryMetrics;
    fn read_io_network_metrics(&self) -> IoNetworkMetrics;
}


Then:

StatusDaemon (and eventually a “HealthDaemon”) uses this trait instead of the pure sim.

SimulationDaemon can be:

turned off (sim level off),

left on for extra synthetic chaos,

or replaced later by a “RealEventsDaemon” that reacts to actual OS events.

Phase 1 can still fake telemetry underneath this trait, but the architecture will be ready to plug into real system calls.

7. Example Real-World Flows
7.1 CPU Overheat Scenario

Telemetry sees:

CPU temp ~95°C, near throttle limit.

Frequent throttling events.

compute_cortex_health:

Decreases Cortex health from, say, 0.92 → 0.75 over time.

Awareness drop:

Awareness index recalculated, e.g. 0.90 → 0.78.

Alerts:

Alert raised: Cortex degraded (thermal).

AiDaemon / policy (future):

Suggests or automatically:

reduces non-critical workloads.

slows simulation intensity.

triggers backup or snapshot if risk persists.

7.2 Memory Pressure Scenario

Telemetry sees:

RAM at 92%+ for extended period.

Heavy swap usage.

compute_memory_health:

Decreases Memory health 0.95 → 0.55.

Awareness:

Awareness index drops to reflect risk to state persistence.

Alerts:

Memory impaired (pressure) warning in shell and logs.

Policy (future):

Suggests / enforces lower cache sizes or moving workloads off node.

8. Phase 1 Implementation Steps (High Level)

Documented mapping (this doc)

✅ Done: conceptual mapping between sim and real subsystems.

Telemetry interface

Introduce a telemetry module with traits and metric structs.

For now, metrics can be derived from the same topology (so we don’t break anything).

Health computation functions

Move Cortex/Memory/IoBridge health calculation into dedicated functions that accept telemetry metrics rather than directly mutating Organ.health.

Status daemon uses those functions

Instead of directly reading/modifying organ health, StatusDaemon:

pulls telemetry

computes health

writes back into the topology.

SimulationDaemon → optional

Keep sim but treat it as:

a source of extra events (e.g. load spikes) OR

a dev tool that can be enabled/disabled for testing in absence of real telemetry.

Start wiring real metrics (future step)

On a chosen host (later), implement a real TelemetryProvider that:

reads OS metrics (CPU temp, RAM, IO, network).

populates metric structs.

Swap simulation metrics for real ones behind the same trait.

9. Long-Term Vision Hook

Even though Phase 1 runs in a simulated or semi-simulated environment, the design already assumes:

AION has a body (organs + health).

AION tracks a sense of self (awareness index).

AION is capable of remembering injuries across reboots (save state / load state).

We will eventually:

plug in real telemetry,

add real AI policy to the Cortex,

and move AION closer to being the primary OS / orchestrator instead of just a process.

This mapping document is the contract: when we attach AION to real hardware and an actual low-level runtime, we know exactly where to plug signals in and how they should influence the organism.


