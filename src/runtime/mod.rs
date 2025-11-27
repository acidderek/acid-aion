use crate::kernel::{self, Bus, PulseKind};

pub fn start() {
    kernel::boot();

    let mut bus = Bus::new();

    // Runtime announces itself to the kernel via the bus.
    bus.emit(
        PulseKind::Command,
        "runtime",
        "hello from the AION runtime (Rust)",
    );

    println!("[AION-RUNTIME] Handing control to kernel loop.");
    println!("[AION-RUNTIME] Press Ctrl+C to stop.\n");

    kernel::run_loop(bus);
}

