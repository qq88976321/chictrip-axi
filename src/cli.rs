//! Command-line surface: the clap tree, the shared context, and dispatch.

use crate::api::Client;
use crate::auth;
use crate::commands;
use crate::error::AxiError;
use crate::output::Document;
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

/// One sentence of what this AXI does; the home view and `--help` share it.
pub const DESCRIPTION: &str = "Agent-first CLI for chicTrip: search places, read expert itineraries, build trips in your account";

/// Every command accepts these, and they are never reported as unknown flags.
pub const GLOBAL_FLAGS: [&str; 5] = ["--fields", "--help", "--json", "--timeout", "--token"];

#[derive(Debug, Parser)]
#[command(name = "chictrip-axi", version, about = DESCRIPTION, long_about = None)]
#[command(
    after_help = "Run with no arguments for the home view: identity, your trips, and the command index."
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct GlobalArgs {
    /// Print the same document as one line of compact JSON
    #[arg(long, global = true)]
    pub json: bool,
    /// Restrict the output to these fields, in this order
    #[arg(long, global = true, value_name = "A,B,C", value_delimiter = ',')]
    pub fields: Option<Vec<String>>,
    /// Seconds to wait for the API
    #[arg(long, global = true, value_name = "SECS", default_value_t = 30)]
    pub timeout: u64,
    /// Use this member token instead of the stored or guest one
    #[arg(long, global = true, value_name = "JWT")]
    pub token: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Store, inspect, and forget the chicTrip member token
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// My trips: list, create, inspect, fill, and delete; preview any trip by id
    Trip {
        #[command(subcommand)]
        command: TripCommand,
    },
    /// Expert itineraries published on chicTrip
    Tour {
        #[command(subcommand)]
        command: TourCommand,
    },
    /// Places: search by keyword, read one in detail
    Poi {
        #[command(subcommand)]
        command: PoiCommand,
    },
    /// Destinations: the country,city,area keys that trip create files a trip under
    Location {
        #[command(subcommand)]
        command: LocationCommand,
    },
    /// Agent integrations: the installable skill file and the Claude Code hook
    Setup {
        #[command(subcommand)]
        command: SetupCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SetupCommand {
    /// Write the agent skill file (--check verifies it instead)
    #[command(after_help = "Examples:
  chictrip-axi setup skill
  chictrip-axi setup skill --check
  chictrip-axi setup skill --out build/SKILL.md
--check exits 1 with error conflict when the file differs from this build, which is how CI gates it.")]
    Skill {
        /// Verify the file instead of writing it
        #[arg(long)]
        check: bool,
        /// Where to write the skill
        #[arg(
            long,
            value_name = "PATH",
            default_value = "skills/chictrip-axi/SKILL.md"
        )]
        out: PathBuf,
    },
    /// Install the Claude Code SessionStart hook that prints the home view
    #[command(after_help = "Examples:
  chictrip-axi setup hooks
  chictrip-axi setup hooks --user
  chictrip-axi setup hooks --remove
Edits .claude/settings.json under the current directory, or ~/.claude/settings.json with --user. Nothing else ever registers a hook.")]
    Hooks {
        /// Edit ~/.claude/settings.json instead of this project's
        #[arg(long)]
        user: bool,
        /// Uninstall the hook instead of installing it
        #[arg(long)]
        remove: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum LocationCommand {
    /// Find destination keys (country,city,area) by place name
    #[command(after_help = "Examples:
  chictrip-axi location search Tokyo
  chictrip-axi location search Kamakura --limit 5")]
    Search {
        keyword: String,
        /// Rows to print (1-200)
        #[arg(long, value_name = "N", default_value_t = 20)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Store a member token copied from the browser
    #[command(after_help = "Examples:
  In the browser console on chictrip.com.tw, run
    copy(JSON.stringify({accessToken:localStorage.accessToken,refreshToken:localStorage.refreshToken,memberId:localStorage.memberId}))
  then paste it into:
    chictrip-axi auth set --from-json -
  Or pass the three values directly:
    chictrip-axi auth set --access-token eyJ... --refresh-token eyJ... --member-id 6bea8a8c-...")]
    Set {
        /// Read the browser JSON from this file, or from stdin with -
        #[arg(long, value_name = "FILE", conflicts_with_all = ["access_token", "refresh_token", "member_id"])]
        from_json: Option<String>,
        #[arg(long, value_name = "JWT", requires = "member_id")]
        access_token: Option<String>,
        #[arg(long, value_name = "JWT")]
        refresh_token: Option<String>,
        #[arg(long, value_name = "UUID")]
        member_id: Option<String>,
    },
    /// Show which token is in use and whether it works
    #[command(after_help = "Examples:
  chictrip-axi auth status
  chictrip-axi auth status --json")]
    Status,
    /// Forget the stored member token
    #[command(after_help = "Examples:
  chictrip-axi auth clear")]
    Clear,
}

#[derive(Debug, Subcommand)]
pub enum TripCommand {
    /// My trips, newest first
    #[command(after_help = "Examples:
  chictrip-axi trip list
  chictrip-axi trip list --limit 5
  chictrip-axi trip list --fields id,name,traffic,update_time")]
    List {
        /// Rows to print (1-200)
        #[arg(long, value_name = "N", default_value_t = 20)]
        limit: usize,
    },
    /// Create an empty trip
    #[command(after_help = "Examples:
  chictrip-axi trip create --name \"Tokyo temples\" --start 2026-10-01 --end 2026-10-03
  chictrip-axi trip create --name \"Kyoto\" --start 2026/11/01 --end 2026/11/04 --traffic Transit --location 7,9,0
Re-running with the same name and dates returns the existing trip; --duplicate forces a second one.")]
    Create {
        #[arg(long, value_name = "NAME")]
        name: String,
        /// First day, YYYY-MM-DD or YYYY/MM/DD
        #[arg(long, value_name = "DATE")]
        start: String,
        /// Last day, inclusive
        #[arg(long, value_name = "DATE")]
        end: String,
        /// Custom, Transit, Driving, Walk, or PublicTransport
        #[arg(long, value_name = "MODE", default_value = "Custom")]
        traffic: String,
        /// Destination key like 7,7,0 (country,city,area) from `location search`; at least one, repeatable
        #[arg(long, value_name = "KEY", required = true)]
        location: Vec<String>,
        /// Create another trip even if one with the same name and dates exists
        #[arg(long)]
        duplicate: bool,
    },
    /// Stops of a trip day by day
    #[command(after_help = "Examples:
  chictrip-axi trip view 3c1d0a2e-0000-0000-0000-000000000000
  chictrip-axi trip view 3c1d0a2e-0000-0000-0000-000000000000 --day 1")]
    View {
        trip_id: String,
        /// Keep only this day
        #[arg(long, value_name = "N")]
        day: Option<i64>,
    },
    /// Stops of any trip by id, readable without signing in
    #[command(after_help = "Examples:
  chictrip-axi trip preview <trip-id>
  chictrip-axi trip preview <trip-id> --day 1
The id is the preViewTravelId of a chicTrip share link
https://www.chictrip.com.tw/?action=preView&preViewTravelId=<trip-id>
chicTrip answers for any trip id, shared or not.")]
    Preview {
        trip_id: String,
        /// Keep only this day
        #[arg(long, value_name = "N")]
        day: Option<i64>,
    },
    /// Append POIs to a day, skipping ones already there
    #[command(after_help = "Examples:
  chictrip-axi trip add <trip-id> --day 1 --poi 8a48a94c-495f-44da-be0d-e1d7564f2b07
  chictrip-axi trip add <trip-id> --day 2 --poi <poi-id> --poi <poi-id> --position best")]
    Add {
        trip_id: String,
        #[arg(long, value_name = "N")]
        day: i64,
        /// POI id from `poi search`; repeatable, added in order
        #[arg(long = "poi", value_name = "POI-ID", required = true)]
        poi: Vec<String>,
        /// last appends to the end of the day, best uses chicTrip's suggestion
        #[arg(long, value_name = "WHERE", default_value = "last")]
        position: String,
        /// Add a POI even when the day already contains it
        #[arg(long)]
        allow_duplicate: bool,
    },
    /// Remove stops by their tsd_id
    #[command(after_help = "Examples:
  chictrip-axi trip remove <trip-id> --stop <tsd-id>
  chictrip-axi trip remove <trip-id> --stop <tsd-id> --stop <tsd-id>
tsd_id comes from `chictrip-axi trip view <trip-id>`.")]
    Remove {
        trip_id: String,
        #[arg(long = "stop", value_name = "TSD-ID", required = true)]
        stop: Vec<String>,
    },
    /// Delete a whole trip
    #[command(after_help = "Examples:
  chictrip-axi trip delete <trip-id>
Deleting a trip that is already gone succeeds as a no-op.")]
    Delete { trip_id: String },
}

#[derive(Debug, Subcommand)]
pub enum TourCommand {
    /// Popular expert itineraries
    #[command(after_help = "Examples:
  chictrip-axi tour list
  chictrip-axi tour list --limit 5
  chictrip-axi tour list --curated")]
    List {
        /// Editor picks instead of the popularity ranking (no paging)
        #[arg(long)]
        curated: bool,
        /// Page of the ranking, 1-based
        #[arg(long, value_name = "N", default_value_t = 1)]
        page: usize,
        /// Rows to print (1-200)
        #[arg(long, value_name = "N", default_value_t = 20)]
        limit: usize,
    },
    /// An expert itinerary day by day
    #[command(after_help = "Examples:
  chictrip-axi tour view 8c9b156f-7990-4d14-951e-a5e4ce3bc575
  chictrip-axi tour view 8c9b156f-7990-4d14-951e-a5e4ce3bc575 --day 1
  chictrip-axi tour view 8c9b156f-7990-4d14-951e-a5e4ce3bc575 --full")]
    View {
        tour_id: String,
        #[arg(long, value_name = "N")]
        day: Option<i64>,
        /// Notes, traffic, highlights, and the whole introduction
        #[arg(long)]
        full: bool,
    },
    /// Copy an expert itinerary into my trips
    #[command(after_help = "Examples:
  chictrip-axi tour copy 8c9b156f-7990-4d14-951e-a5e4ce3bc575
chicTrip allows several copies of the same tour, so this is not idempotent.")]
    Copy { tour_id: String },
}

#[derive(Debug, Subcommand)]
pub enum PoiCommand {
    /// Find places and their ids
    #[command(after_help = "Examples:
  chictrip-axi poi search \"Senso-ji\"
  chictrip-axi poi search ramen --near 35.7111,139.7963
  chictrip-axi poi search ramen --limit 5 --fields id,name,lat,lng")]
    Search {
        keyword: String,
        /// Bias the search around LAT,LNG
        #[arg(long, value_name = "LAT,LNG")]
        near: Option<String>,
        /// Rows to print (1-200)
        #[arg(long, value_name = "N", default_value_t = 20)]
        limit: usize,
    },
    /// Address, hours, rating, and description of a place
    #[command(after_help = "Examples:
  chictrip-axi poi view 8a48a94c-495f-44da-be0d-e1d7564f2b07
  chictrip-axi poi view 8a48a94c-495f-44da-be0d-e1d7564f2b07 --full")]
    View {
        poi_id: String,
        /// The whole description plus media and ticket tables
        #[arg(long)]
        full: bool,
    },
}

/// What a command needs from the invocation: the global flags plus the
/// lazily built client, so `--help` and validation never touch the network.
pub struct Context {
    pub global: GlobalArgs,
}

impl Context {
    pub fn new(global: GlobalArgs) -> Self {
        Context { global }
    }

    pub fn client(&self) -> Result<Client, AxiError> {
        let creds = auth::resolve(self.global.token.as_deref())?;
        Ok(Client::new(creds, self.global.timeout))
    }

    /// The guest token answers member endpoints with the demo account's data,
    /// so refuse locally instead of returning something plausible and wrong.
    pub fn member_client(&self, what: &str) -> Result<Client, AxiError> {
        let client = self.client()?;
        if client.is_guest() {
            return Err(AxiError::auth_required(format!(
                "{what} needs a chicTrip member token"
            )));
        }
        Ok(client)
    }
}

pub fn run(cli: Cli) -> Result<Document, AxiError> {
    let ctx = Context::new(cli.global.clone());
    let mut doc = match &cli.command {
        None => commands::home::run(&ctx)?,
        Some(Command::Auth { command }) => commands::auth::run(&ctx, command)?,
        Some(Command::Trip { command }) => commands::trip::run(&ctx, command)?,
        Some(Command::Tour { command }) => commands::tour::run(&ctx, command)?,
        Some(Command::Poi { command }) => commands::poi::run(&ctx, command)?,
        Some(Command::Location { command }) => commands::location::run(&ctx, command)?,
        Some(Command::Setup { command }) => commands::setup::run(command)?,
    };
    if let Some(fields) = &cli.global.fields {
        doc.apply_fields(fields)?;
    }
    Ok(doc)
}

/// The valid flags of the deepest subcommand named in `argv`, for the
/// self-correcting usage error.
pub fn flags_for_argv(argv: &[String]) -> Vec<String> {
    let mut command = Cli::command();
    let mut at_root = true;
    for arg in argv.iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        match command.find_subcommand(arg) {
            Some(sub) => {
                command = sub.clone();
                at_root = false;
            }
            None => break,
        }
    }
    let mut flags: Vec<String> = command
        .get_arguments()
        .filter_map(|arg| arg.get_long().map(|long| format!("--{long}")))
        .collect();
    flags.extend(GLOBAL_FLAGS.iter().map(|f| f.to_string()));
    if at_root {
        flags.push("--version".to_string());
    }
    flags.sort();
    flags.dedup();
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn flags_are_reported_for_the_deepest_named_subcommand() {
        let argv: Vec<String> = ["chictrip-axi", "auth", "set", "--bogus"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let flags = flags_for_argv(&argv);
        assert!(flags.contains(&"--from-json".to_string()));
        assert!(flags.contains(&"--access-token".to_string()));
        assert!(flags.contains(&"--json".to_string()));
        assert!(!flags.contains(&"--version".to_string()));
    }

    #[test]
    fn the_root_reports_its_own_flags() {
        let argv: Vec<String> = ["chictrip-axi", "--bogus"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let flags = flags_for_argv(&argv);
        assert!(flags.contains(&"--version".to_string()));
        assert!(flags.contains(&"--timeout".to_string()));
    }

    #[test]
    fn global_flags_parse_after_a_subcommand() {
        let cli = Cli::try_parse_from(["chictrip-axi", "auth", "status", "--json"]).unwrap();
        assert!(cli.global.json);
    }

    #[test]
    fn setup_skill_reports_its_own_flags_and_the_globals() {
        let argv: Vec<String> = ["chictrip-axi", "setup", "skill", "--bogus"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            flags_for_argv(&argv),
            [
                "--check",
                "--fields",
                "--help",
                "--json",
                "--out",
                "--timeout",
                "--token"
            ]
        );
    }

    #[test]
    fn poi_search_reports_its_own_flags_and_the_globals() {
        let argv: Vec<String> = ["chictrip-axi", "poi", "search", "--bogus"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let flags = flags_for_argv(&argv);
        assert_eq!(
            flags,
            [
                "--fields",
                "--help",
                "--json",
                "--limit",
                "--near",
                "--timeout",
                "--token"
            ]
        );
    }
}
