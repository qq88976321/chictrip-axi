//! `chictrip-axi` entry point: parse arguments, dispatch, render the document
//! on stdout, map errors to the AXI exit codes.

use anyhow::Result;
use chictrip_axi::cli::{self, Cli};
use chictrip_axi::error::AxiError;
use chictrip_axi::output::render;
use clap::Parser;
use clap::error::ErrorKind;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    match run(&argv) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let error = e
                .downcast_ref::<AxiError>()
                .cloned()
                .unwrap_or_else(|| AxiError::internal(format!("{e:#}")));
            print!("{}", render(&error.document(), wants_json(&argv)));
            error.exit_code()
        }
    }
}

fn run(argv: &[String]) -> Result<()> {
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(e) => return Err(clap_error(e, argv).into()),
    };
    let json = cli.global.json;
    let document = cli::run(cli)?;
    print!("{}", render(&document, json));
    Ok(())
}

/// `--help` and `--version` are clap "errors" that succeeded; everything else
/// becomes the AXI usage error with the command's flags inline, so the agent
/// self-corrects without a second call.
fn clap_error(error: clap::Error, argv: &[String]) -> AxiError {
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ) {
        print!("{error}");
        std::process::exit(0);
    }
    AxiError::usage(first_paragraph(&error.render().to_string()))
        .with_flags(cli::flags_for_argv(argv))
        .with_help([format!(
            "Run `{} --help` for the flags and defaults and examples",
            command_path(argv)
        )])
}

/// clap puts the offending names on the lines after a colon-terminated
/// headline, so the whole first paragraph is the message.
fn first_paragraph(rendered: &str) -> String {
    let paragraph: Vec<&str> = rendered
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect();
    if paragraph.is_empty() {
        return "invalid arguments".to_string();
    }
    paragraph
        .join(" ")
        .trim_start_matches("error:")
        .trim()
        .to_string()
}

fn command_path(argv: &[String]) -> String {
    let mut path = vec!["chictrip-axi".to_string()];
    for arg in argv.iter().skip(1).take(2) {
        if arg.starts_with('-') {
            break;
        }
        path.push(arg.clone());
    }
    path.join(" ")
}

/// The error document honours `--json` even when parsing never got that far.
fn wants_json(argv: &[String]) -> bool {
    argv.iter().any(|arg| arg == "--json")
}
