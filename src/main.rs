mod kernel;
mod organism;
mod telemetry;
mod http;
pub mod capabilities;
pub mod memory;


fn main() {
    kernel::boot();
    let bus = kernel::Bus::new();
    kernel::run_loop(bus);
}
