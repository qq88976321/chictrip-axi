//! chictrip-axi: an agent-first (AXI) command-line client for the chicTrip
//! travel API.
//!
//! Thin binary (`src/main.rs`) over this library so the pure parts (the CLI
//! definition, the output layer, the apiStatus mapping) are unit tested
//! without spawning a process.

pub mod api;
pub mod auth;
pub mod cli;
pub mod commands;
pub mod datetime;
pub mod error;
pub mod output;
