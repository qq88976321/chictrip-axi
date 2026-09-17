//! `location search`: the destination keys `trip create` files a trip under.

use super::{count_line, validate_limit, value};
use crate::api::types::LocationHit;
use crate::cli::{Context, LocationCommand};
use crate::error::AxiError;
use crate::output::{Document, Table};

pub fn run(ctx: &Context, command: &LocationCommand) -> Result<Document, AxiError> {
    match command {
        LocationCommand::Search { keyword, limit } => search(ctx, keyword, *limit),
    }
}

fn search(ctx: &Context, keyword: &str, limit: usize) -> Result<Document, AxiError> {
    let limit = validate_limit(limit)?;
    if keyword.trim().is_empty() {
        return Err(AxiError::usage("the keyword is empty"));
    }
    let client = ctx.client()?;
    let data = client.get(
        "ExpertTour/SearchLocation",
        &[("keyword", keyword.to_string())],
    )?;
    let hits: Vec<LocationHit> = serde_json::from_value(data)
        .map_err(|e| AxiError::internal(format!("chicTrip sent an unexpected shape: {e}")))?;
    let total = hits.len();
    let shown = total.min(limit);

    let mut table = Table::new(&["name", "full_name", "key"], &[]);
    for hit in hits.iter().take(shown) {
        table.push(&[
            ("name", value(&hit.name)),
            ("full_name", value(&hit.full_name)),
            ("key", value(&hit.location_key)),
        ]);
    }

    let mut doc = Document::new();
    doc.set("count", count_line(shown, total));
    doc.set_table("locations", table);
    doc.set_primary("locations");
    if total == 0 {
        doc.set_strings(
            "help",
            &[format!(
                "No destination matches \"{keyword}\"; try a shorter or more common place name"
            )],
        );
    } else {
        doc.set_strings(
            "help",
            &["Run `chictrip-axi trip create --name \"<name>\" --start <date> --end <date> --location <key>` to file a trip there"],
        );
    }
    Ok(doc)
}
