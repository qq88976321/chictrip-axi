//! The bare-invocation home view: identity, live content, command index.

use super::{count, trips_table, value};
use crate::api::trips;
use crate::api::types::PopularRanking;
use crate::auth::display_path;
use crate::cli::{Context, DESCRIPTION};
use crate::error::AxiError;
use crate::output::{Document, Table};

const HOME_ROWS: usize = 5;

/// The static half of the home view, and the single source the
/// `setup skill` generator reads.
pub const COMMAND_INDEX: &[(&str, &str)] = &[
    ("auth set", "Store a member token copied from the browser"),
    (
        "auth status",
        "Show which token is in use and whether it works",
    ),
    ("auth clear", "Forget the stored member token"),
    ("trip list", "My trips (newest first)"),
    ("trip create --name --start --end", "Create an empty trip"),
    (
        "trip view <trip-id>",
        "Stops of a trip day by day (--full for notes and legs)",
    ),
    (
        "trip preview <trip-id>",
        "Stops of any trip by id (no login needed)",
    ),
    (
        "trip add <trip-id> --day N --poi ID...",
        "Add POIs to a day (skips duplicates)",
    ),
    ("trip remove <trip-id> --stop ID...", "Remove stops"),
    ("trip delete <trip-id>", "Delete a whole trip"),
    (
        "tour list",
        "Popular expert itineraries (--curated for editor picks)",
    ),
    ("tour view <tour-id>", "An expert itinerary day by day"),
    (
        "tour copy <tour-id>",
        "Copy an expert itinerary into my trips",
    ),
    ("poi search <keyword>", "Find places and their ids"),
    (
        "poi view <poi-id>",
        "Address and hours and rating and description of a place",
    ),
    (
        "location search <keyword>",
        "Destination keys for trip create",
    ),
    (
        "setup skill",
        "Write the agent skill file (--check verifies it)",
    ),
    (
        "setup hooks",
        "Install the Claude Code SessionStart hook on request",
    ),
];

pub fn run(ctx: &Context) -> Result<Document, AxiError> {
    let mut doc = Document::new();
    doc.set("bin", binary_path());
    doc.set("description", DESCRIPTION);

    let client = ctx.client()?;
    let guest = client.is_guest();
    doc.set("auth", if guest { "guest" } else { "member" });

    let live = if guest {
        popular_tours(&client)
    } else {
        my_trips(&client)
    };
    match live {
        Ok((key, table)) => {
            doc.set_table(key, table);
            doc.set_primary(key);
        }
        Err(error) => {
            doc.set("error", error.code.as_str());
            doc.set("message", error.message.as_str());
            doc.set_table("commands", command_index());
            doc.set_strings("help", &home_help(guest));
            return Err(AxiError::new(error.code, error.message)
                .with_request_id(error.request_id)
                .with_help(home_help(guest))
                .with_document(doc));
        }
    }

    doc.set_table("commands", command_index());
    doc.set_strings("help", &home_help(guest));
    Ok(doc)
}

fn home_help(guest: bool) -> Vec<String> {
    let first = if guest {
        "Run `chictrip-axi auth set --from-json -` and paste the JSON copied from the browser to see my own trips"
    } else {
        "Run `chictrip-axi trip view <id>` to continue a trip above"
    };
    vec![
        first.to_string(),
        "Run `chictrip-axi <command> --help` for flags and defaults and examples".to_string(),
    ]
}

fn command_index() -> Table {
    let mut table = Table::new(&["command", "summary"], &[]);
    for (command, summary) in COMMAND_INDEX.iter().copied() {
        table.push(&[("command", command.into()), ("summary", summary.into())]);
    }
    table
}

fn my_trips(client: &crate::api::Client) -> Result<(&'static str, Table), AxiError> {
    let mut trips = trips::list_trips(client)?;
    trips.truncate(HOME_ROWS);
    let mut table = trips_table(&trips);
    table.select(&[
        "id".into(),
        "name".into(),
        "start".into(),
        "end".into(),
        "days".into(),
    ])?;
    Ok(("trips", table))
}

fn popular_tours(client: &crate::api::Client) -> Result<(&'static str, Table), AxiError> {
    let data = client.get(
        "ExpertTour/PopularRanking",
        &[
            ("page", "1".to_string()),
            ("pageSize", HOME_ROWS.to_string()),
        ],
    )?;
    let ranking: PopularRanking = serde_json::from_value(data)
        .map_err(|e| AxiError::internal(format!("chicTrip sent an unexpected shape: {e}")))?;
    let mut table = Table::new(&["id", "name", "destination", "expert", "likes"], &[]);
    for tour in ranking.expert_tour_detail_list.iter().take(HOME_ROWS) {
        table.push(&[
            ("id", value(&tour.travel_schedule_id)),
            ("name", value(&tour.name)),
            ("destination", value(&tour.destination)),
            ("expert", value(&tour.expert_name)),
            ("likes", count(tour.like_count)),
        ]);
    }
    Ok(("tours", table))
}

fn binary_path() -> String {
    std::env::current_exe()
        .map(|path| display_path(&path))
        .unwrap_or_else(|_| "chictrip-axi".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_index_renders_as_one_table() {
        let mut doc = Document::new();
        doc.set_table("commands", command_index());
        let header = format!("commands[{}]{{command,summary}}:", COMMAND_INDEX.len());
        assert!(crate::output::render(&doc, false).contains(&header));
    }
}
