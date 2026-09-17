//! chictrip-axi: an agent-first (AXI) command-line client for the chicTrip
//! travel API.
//!
//! Thin binary (`src/main.rs`) over this library so the pure parts (the
//! CLI definition, the home view, the error-to-exit-code mapping) are unit
//! tested without spawning a process. Modules are declared here as they
//! are implemented.

pub mod cli;
pub mod error;
