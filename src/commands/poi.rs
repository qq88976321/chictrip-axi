//! `poi search` and `poi view`.

use super::{count, count_line, number, parse_near, validate_limit, value};
use crate::api::types::PoiSearchResult;
use crate::cli::{Context, PoiCommand};
use crate::error::AxiError;
use crate::output::{Document, Table, truncate};
use serde_json::Value;

pub fn run(ctx: &Context, command: &PoiCommand) -> Result<Document, AxiError> {
    match command {
        PoiCommand::Search {
            keyword,
            near,
            limit,
        } => search(ctx, keyword, near.as_deref(), *limit),
        PoiCommand::View { poi_id, full } => view(ctx, poi_id, *full),
    }
}

fn search(
    ctx: &Context,
    keyword: &str,
    near: Option<&str>,
    limit: usize,
) -> Result<Document, AxiError> {
    let limit = validate_limit(limit)?;
    if keyword.trim().is_empty() {
        return Err(AxiError::usage("the search keyword is empty"));
    }
    let (lat, lng) = match near {
        Some(near) => parse_near(near)?,
        None => (0.0, 0.0),
    };

    let client = ctx.client()?;
    let data = client.get(
        "PoiSearch/SearchByKeyword",
        &[
            ("keyword", keyword.to_string()),
            ("centerLongitude", lng.to_string()),
            ("centerLatitude", lat.to_string()),
        ],
    )?;
    let found: PoiSearchResult = serde_json::from_value(data)
        .map_err(|e| AxiError::internal(format!("chicTrip sent an unexpected shape: {e}")))?;

    let total = found.result.len();
    let shown = total.min(limit);
    let mut table = Table::new(
        &["id", "name", "category", "rating", "city", "area"],
        &[
            "reviews",
            "favorites",
            "visits",
            "lat",
            "lng",
            "address",
            "place_id",
            "country",
        ],
    );
    for poi in found.result.iter().take(shown) {
        table.push(&[
            ("id", value(&poi.id)),
            ("name", value(&poi.name)),
            ("category", value(&poi.category_type)),
            ("rating", number(poi.rating_count)),
            ("city", value(&poi.location.location_city_name)),
            ("area", value(&poi.location.location_area_name)),
            ("reviews", count(poi.rating_total)),
            ("favorites", count(poi.favorite_count)),
            ("visits", count(poi.join_count)),
            ("lat", number(poi.latitude)),
            ("lng", number(poi.longitude)),
            ("address", value(&poi.address)),
            ("place_id", value(&poi.place_id)),
            ("country", value(&poi.location.location_country_name)),
        ]);
    }

    let mut doc = Document::new();
    doc.set("count", count_line(shown, total));
    doc.set_table("pois", table);
    doc.set_primary("pois");
    if found.has_next_page {
        doc.set("more", true);
    }
    if total == 0 {
        doc.set_strings(
            "help",
            &[
                "Run `chictrip-axi poi search \"<shorter keyword>\"` with fewer words",
                "Run `chictrip-axi poi search \"<keyword>\" --near <lat>,<lng>` to search around a point",
            ],
        );
    } else {
        doc.set_strings(
            "help",
            &[
                "Run `chictrip-axi poi view <id>` for hours and address and description",
                "Run `chictrip-axi trip add <trip-id> --day <n> --poi <id>` to put one in a trip",
            ],
        );
    }
    Ok(doc)
}

fn view(ctx: &Context, poi_id: &str, full: bool) -> Result<Document, AxiError> {
    let client = ctx.client()?;
    let poi = crate::api::trips::poi_detail(&client, poi_id)?;

    let mut detail = Document::new();
    detail.set("id", value(&poi.id));
    detail.set("name", value(&poi.name));
    detail.set("category", value(&poi.category_type));
    detail.set("rating", number(poi.rating_count));
    detail.set("reviews", count(poi.rating_total));
    detail.set("favorites", count(poi.favorite_count));
    detail.set("visits", count(poi.join_count));
    detail.set("address", value(&poi.address));
    detail.set("country", value(&poi.location.location_country_name));
    detail.set("city", value(&poi.location.location_city_name));
    detail.set("area", value(&poi.location.location_area_name));
    detail.set("lat", number(poi.latitude));
    detail.set("lng", number(poi.longitude));
    detail.set("phone", value(&poi.phone_number));
    detail.set("url", value(&poi.url));

    let hours: Vec<Value> = poi
        .open_times
        .iter()
        .filter_map(|open| {
            let code = open.code.clone()?;
            let descript = open.descript.clone().unwrap_or_default();
            Some(Value::from(format!("{code} {descript}").trim().to_string()))
        })
        .collect();
    if !hours.is_empty() {
        detail.set_list("hours", hours);
    }
    let tags = poi.tag_names();
    if !tags.is_empty() {
        detail.set_strings("tags", &tags);
    }

    let description = poi.description.clone().unwrap_or_default();
    let mut truncated = false;
    if !description.is_empty() {
        if full {
            detail.set("description", description.as_str());
        } else {
            let (text, cut) = truncate(&description);
            truncated = cut;
            detail.set("description", text);
        }
    }
    detail.set("media", poi.media.len() as i64);
    detail.set("tickets", poi.poi_tickets.len() as i64);

    let mut doc = Document::new();
    doc.set_object("poi", detail);
    doc.set_primary("poi");

    if full {
        let mut media = Table::new(&["type", "source", "url"], &[]);
        for item in &poi.media {
            media.push(&[
                ("type", value(&item.media_type)),
                ("source", value(&item.source)),
                ("url", value(&item.url)),
            ]);
        }
        doc.set_table("media", media);
        let mut tickets = Table::new(&["title", "price", "currency", "url"], &[]);
        for ticket in &poi.poi_tickets {
            tickets.push(&[
                ("title", value(&ticket.title)),
                ("price", number(ticket.price)),
                ("currency", value(&ticket.currency)),
                ("url", value(&ticket.url)),
            ]);
        }
        doc.set_table("tickets", tickets);
    } else if truncated || !poi.media.is_empty() || !poi.poi_tickets.is_empty() {
        let id = poi.id.clone().unwrap_or_else(|| poi_id.to_string());
        doc.set_strings(
            "help",
            &[format!(
                "Run `chictrip-axi poi view {id} --full` for the full description and media urls and tickets"
            )],
        );
    }
    Ok(doc)
}
