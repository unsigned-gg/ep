//! ep CLI — Escapement scheduler and daemon

use escapement_core::registry::Registry;

fn main() {
    println!("ep (escapement) v1.1.0 — metered, deterministic dispatch engine");
    let reg = Registry::new();
    println!("Registry active with {} registered agents.", reg.len());
}
