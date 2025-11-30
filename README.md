# ACID-AION

AION is a modular, distributed operating system designed as a biological-inspired compute organism.  
Phase 1 focuses on the bootstrapped microkernel, messaging model, and command interface.

## Structure
- kernel/  — Phase 1 microkernel prototype (Go)
- runtime/ — AION runtime environment (Go)
- sim/     — Hardware-free simulation sandbox
- tests/   — Unit and integration tests
- docs/    — All architecture, phase, and planning documents


AION – Artificial Intelligence Operating Nexus
An AI-First OS Kernel Prototype

AION is an experimental operating-system kernel built from the premise that AI is the root abstraction, not an application on top.

The goal is to explore a world where:

AI is the primary system supervisor

Every subsystem expresses itself as an organ with capabilities

A shared MemoryBus replaces traditional config/state

Awareness drives scheduling, load selection, and self-protection

The system is introspectable through a built-in HTTP interface

The kernel behaves like a small organism, not a Unix clone

AION is not Linux, not microkernel, not monolithic — it’s a new model.

🌐 Features (Phase 1 Completed)
✔ Organ-Based Kernel Architecture

The OS is structured as:

Nodes – compute or IO points

Organs – functional system units (Cortex, Memory, IoBridge…)

Peripherals – CPU, GPU, NVMe, NIC, Display

Capabilities – Compute, Storage, Perception, Networking

✔ Telemetry-Driven Health

Each organ has a health: f32 value (0.0–1.0).
Telemetry (real or simulated) updates organ health:

CPU/GPU load

RAM pressure

IO latency

Network packet loss

✔ Awareness Index (System Consciousness Model)

AION computes:

awareness = 0.4*cortex + 0.3*memory + 0.3*io


Then assigns a label:

optimal

stable

impaired

critical

unconscious

This value drives AI policy and system behavior.

✔ AI Cortex Daemon

A tiny AI brain runs every 2 seconds:

reads awareness

selects a system policy

writes its thoughts into the MemoryBus

affects simulation-level behavior

Example policies:

policy=push_capacity

policy=maintain_load

policy=reduce_load

policy=protect_core(sim_off)

✔ MemoryBus — Global Neural Scratchpad

Shared key/value memory used by:

kernel

AI Cortex

daemons

shell commands

HTTP API

Keys include:

cortex.policy

cortex.awareness

kernel.last_status

✔ Built-in HTTP Introspection Server

Runs at:

http://127.0.0.1:8080


Endpoints:

/ – HTML homepage

/status – health and awareness

/metrics – CPU/MEM/IO telemetry snapshot

No external crates except tiny_http.

✔ AION Shell

Type commands inside the kernel:

help
status
topology
nodes
organs
peripherals
health
alerts
metrics
mem
mem get <key>
mem set <key> <value>
sim level <off|low|high>
damage memory 0.1
heal cortex 0.2
save state
load state
quit

✔ Persistent Organ Health

State saved to:

aion_state.txt


Includes health values for every organ.

🧠 Architecture Overview
Core Components

Kernel – manages daemons, bus, topology

Bus – logging + simulation + telemetry mode

Daemon – heartbeat, status, AI cortex, simulation, command

Organism – nodes, organs, peripherals, capabilities

MemoryBus – shared working memory

HTTP Server – exposes metrics & homepage

Telemetry – simulated or real backends

🚀 Running AION
1. Build & run:
cargo run

2. Visit:
http://127.0.0.1:8080

3. Use AION Shell:
AION> status
AION> mem ls
AION> sim level high
AION> damage memory 0.1

📡 Internal Diagram (Text)
 ===================== AION Kernel ======================
 |                                                      |
 |   +----------------------------------------------+   |
 |   |                AI Cortex Daemon              |   |
 |   |  awareness → policy → MemoryBus writes       |   |
 |   +----------------------------------------------+   |
 |                     |                                |
 |                 MemoryBus                             |
 |                     |                                |
 |   +----------------------------------------------+   |
 |   |              Status Daemon                    |   |
 |   |  telemetry → organ health → awareness        |   |
 |   +----------------------------------------------+   |
 |     |         |         |                          |
 |   Cortex    Memory   IoBridge       (Organs)       |
 |                                                      |
 ========================================================

🗺 Roadmap
Phase 2 (Next Up)

Capability Routing Layer

Organ Scheduler

Event Subscription Bus

Device & Peripheral Registry

AION FS (virtual file abstractions over memory & organ health)

Multinode support

Phase 3

Distributed nodes

GPU cluster integration

Memory fabric expansion

Real hardware probing layer

Phase 4

AION desktop environment

Organ UI (visualize organs & health live)

Autonomous repair heuristics

🤝 Contributing

Fork the repo

Create feature branches

Format code with rustfmt

Submit PRs

AION is experimental — ideas welcome.

If you'd like, I can also generate:

✅ Architecture image
✅ Organ diagram
✅ AI Cortex diagram
✅ Developer API documentation
✅ Telemetry spec
✅ AION Shell cheat sheet

Just tell me.