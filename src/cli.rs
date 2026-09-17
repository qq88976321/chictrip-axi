//! Command-line surface: the clap definition and the argument-free home view.

use clap::Parser;

/// One sentence of what this AXI does; the home view and `--help` share it.
pub const DESCRIPTION: &str = "Agent-first CLI for the chicTrip travel API";

/// Agent-first CLI for the chicTrip travel API.
///
/// No subcommands yet: API commands land in the next milestone. clap
/// already gives the AXI basics for free: `--version` prints a bare
/// `chictrip-axi X.Y.Z`, `--help` is concise, and an unknown flag is a
/// usage error with exit code 2.
#[derive(Debug, Parser)]
#[command(name = "chictrip-axi", version, about = DESCRIPTION, long_about = None)]
pub struct Cli {}

/// The bare-invocation home view. AXI asks for live content here rather
/// than help text; until commands exist it says so and points onward.
pub fn home_view(bin: &str) -> String {
    format!(
        "bin: {bin}\n\
         description: {DESCRIPTION}\n\
         commands: 0 (API commands land in the next milestone)\n\
         next: chictrip-axi --help\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn home_view_leads_with_binary_and_description() {
        let view = home_view("/usr/local/bin/chictrip-axi");
        let mut lines = view.lines();
        assert_eq!(lines.next(), Some("bin: /usr/local/bin/chictrip-axi"));
        assert_eq!(
            lines.next(),
            Some(format!("description: {DESCRIPTION}").as_str())
        );
        assert!(view.ends_with('\n'));
    }

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }
}
