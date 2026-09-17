//! `chictrip-axi` entry point: parse arguments, dispatch, map errors to
//! the AXI exit codes.

use anyhow::Result;
use chictrip_axi::cli::{self, Cli};
use chictrip_axi::error::AxiError;
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("chictrip-axi: {e:#}");
            e.downcast_ref::<AxiError>()
                .map_or(ExitCode::FAILURE, AxiError::exit_code)
        }
    }
}

fn run() -> Result<()> {
    let Cli {} = Cli::parse();
    let bin = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "chictrip-axi".to_string());
    print!("{}", cli::home_view(&bin));
    Ok(())
}
