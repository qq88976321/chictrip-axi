//! `tour list`, `tour view`, and `tour copy`.

use super::{count, count_line, number, select_days, stops_table, validate_limit, value};
use crate::api::trips;
use crate::api::types::{PopularRanking, TourDetail, TourListItem};
use crate::cli::{Context, TourCommand};
use crate::error::AxiError;
use crate::output::{Document, Table, truncate};
use serde_json::Value;

pub fn run(ctx: &Context, command: &TourCommand) -> Result<Document, AxiError> {
    match command {
        TourCommand::List {
            curated,
            page,
            limit,
        } => list(ctx, *curated, *page, *limit),
        TourCommand::View { tour_id, day, full } => view(ctx, tour_id, *day, *full),
        TourCommand::Copy { tour_id } => copy(ctx, tour_id),
    }
}

fn list(ctx: &Context, curated: bool, page: usize, limit: usize) -> Result<Document, AxiError> {
    let limit = validate_limit(limit)?;
    if page == 0 {
        return Err(AxiError::usage("--page starts at 1"));
    }
    let client = ctx.client()?;

    let (tours, next_page) = if curated {
        let data = client.get("ExpertTour/Exclusive", &[])?;
        let tours: Vec<TourListItem> = decode(data)?;
        (tours, None)
    } else {
        let data = client.get(
            "ExpertTour/PopularRanking",
            &[("page", page.to_string()), ("pageSize", limit.to_string())],
        )?;
        let ranking: PopularRanking = decode(data)?;
        (ranking.expert_tour_detail_list, ranking.next_page)
    };

    let total = tours.len();
    let shown = total.min(limit);
    let mut table = Table::new(
        &["id", "name", "destination", "expert", "likes", "used"],
        &["tags", "introduction"],
    );
    for tour in tours.iter().take(shown) {
        let tags = tour.tag_list();
        let introduction = tour
            .introduction
            .as_ref()
            .map(|text| truncate(text).0)
            .unwrap_or_default();
        table.push(&[
            ("id", value(&tour.travel_schedule_id)),
            ("name", value(&tour.name)),
            ("destination", value(&tour.destination)),
            ("expert", value(&tour.expert_name)),
            ("likes", count(tour.like_count)),
            ("used", count(tour.used_count)),
            (
                "tags",
                if tags.is_empty() {
                    Value::Null
                } else {
                    Value::from(tags.join("|"))
                },
            ),
            (
                "introduction",
                if introduction.is_empty() {
                    Value::Null
                } else {
                    Value::from(introduction)
                },
            ),
        ]);
    }

    let mut doc = Document::new();
    doc.set("count", count_line(shown, total));
    if !curated {
        doc.set("page", page as i64);
        if let Some(next) = next_page.filter(|n| *n > 0) {
            doc.set("next_page", next);
        }
    }
    doc.set_table("tours", table);
    doc.set_primary("tours");

    let mut help = vec![
        "Run `chictrip-axi tour view <id>` to read an itinerary day by day".to_string(),
        "Run `chictrip-axi tour copy <id>` to copy one into my trips".to_string(),
    ];
    if let Some(next) = next_page.filter(|n| *n > 0) {
        help.push(format!(
            "Run `chictrip-axi tour list --page {next}` for the next page"
        ));
    }
    if total == 0 {
        help = vec!["Run `chictrip-axi tour list --curated` for the editor picks".to_string()];
    }
    doc.set_strings("help", &help);
    Ok(doc)
}

fn view(ctx: &Context, tour_id: &str, day: Option<i64>, full: bool) -> Result<Document, AxiError> {
    let client = ctx.client()?;
    let data = client.get(
        "ExpertTour/TourV2",
        &[("travelScheduleId", tour_id.to_string())],
    )?;
    let tour: TourDetail = decode(data)?;
    let overview = &tour.overview;

    let mut detail = Document::new();
    detail.set("id", value(&overview.travel_schedule_id));
    detail.set("name", value(&overview.name));
    detail.set("destination", value(&overview.destination));
    detail.set("days", count(overview.total_day));
    detail.set("start", value(&overview.start_date));
    detail.set("end", value(&overview.end_date));
    detail.set("expert", value(&overview.expert_name));
    detail.set("likes", count(overview.like_count));
    detail.set("used", count(overview.used_count));
    detail.set("youtube", value(&overview.you_tube_url));

    let introduction = overview.introduction.clone().unwrap_or_default();
    let mut truncated = false;
    if !introduction.is_empty() {
        if full {
            detail.set("introduction", introduction.as_str());
        } else {
            let (text, cut) = truncate(&introduction);
            truncated = cut;
            detail.set("introduction", text);
        }
    }

    let mut doc = Document::new();
    doc.set_object("tour", detail);
    let days = select_days(&tour.day_list, day, overview.total_day)?;
    doc.set_table("stops", stops_table(&days, full, false));
    doc.set_primary("stops");

    if full {
        let mut highlights = Table::new(&["poi_id", "name", "rating", "visits"], &[]);
        for item in &overview.highlight_poi_list {
            highlights.push(&[
                ("poi_id", value(&item.poi_id)),
                ("name", value(&item.poi_name)),
                ("rating", number(item.rating_count)),
                ("visits", count(item.join_count)),
            ]);
        }
        doc.set_table("highlights", highlights);
    }

    let mut help = vec![
        "Run `chictrip-axi poi view <poi_id>` for a stop".to_string(),
        format!("Run `chictrip-axi tour copy {tour_id}` to copy this itinerary into my trips"),
    ];
    if !full && truncated {
        help.push(format!(
            "Run `chictrip-axi tour view {tour_id} --full` for notes and traffic per stop"
        ));
    }
    doc.set_strings("help", &help);
    Ok(doc)
}

fn copy(ctx: &Context, tour_id: &str) -> Result<Document, AxiError> {
    let client = ctx.member_client("tour copy")?;
    trips::copy_tour(&client, tour_id)?;
    let copied = trips::list_trips(&client)?;
    let newest = copied
        .iter()
        .max_by_key(|trip| trip.update_time.unwrap_or(0));

    let mut doc = Document::new();
    let mut trip = Document::new();
    let mut trip_id = String::new();
    if let Some(newest) = newest {
        trip_id = newest.id.clone().unwrap_or_default();
        trip.set("id", value(&newest.id));
        trip.set("name", value(&newest.name));
        trip.set("start", value(&newest.start_date));
        trip.set("end", value(&newest.end_date));
        trip.set("days", count(newest.total_day));
        trip.set("update_time", count(newest.update_time));
    }
    doc.set_object("trip", trip);
    doc.set_primary("trip");
    doc.set("source", tour_id);
    doc.set_strings(
        "help",
        &[
            format!("Run `chictrip-axi trip view {trip_id}` to see the copy"),
            format!("Run `chictrip-axi trip add {trip_id} --day 1 --poi <poi-id>` to extend it"),
        ],
    );
    Ok(doc)
}

fn decode<T: serde::de::DeserializeOwned>(data: Value) -> Result<T, AxiError> {
    serde_json::from_value(data)
        .map_err(|e| AxiError::internal(format!("chicTrip sent an unexpected shape: {e}")))
}
