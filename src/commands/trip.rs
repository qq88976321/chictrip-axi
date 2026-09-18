//! `trip list`, `create`, `view`, `add`, `remove`, and `delete`.

use super::{count, count_line, select_days, stops_table, trips_table, validate_limit, value};
use crate::api::trips::{self, Position};
use crate::api::types::{CreatedTrip, Day, TripDetail};
use crate::cli::{Context, TripCommand};
use crate::datetime::Date;
use crate::error::{AxiError, ErrorCode};
use crate::output::{Document, Table, truncate};
use serde_json::Value;

const TRAFFIC_MODES: [&str; 5] = ["Custom", "Transit", "Driving", "Walk", "PublicTransport"];
const MAX_TRIP_DAYS: i64 = 60;

pub fn run(ctx: &Context, command: &TripCommand) -> Result<Document, AxiError> {
    match command {
        TripCommand::List { limit } => list(ctx, *limit),
        TripCommand::Create {
            name,
            start,
            end,
            traffic,
            location,
            duplicate,
        } => create(ctx, name, start, end, traffic, location, *duplicate),
        TripCommand::View { trip_id, day, full } => view(ctx, trip_id, *day, *full),
        TripCommand::Preview { trip_id, day, full } => preview(ctx, trip_id, *day, *full),
        TripCommand::Add {
            trip_id,
            day,
            poi,
            position,
            allow_duplicate,
        } => add(ctx, trip_id, *day, poi, position, *allow_duplicate),
        TripCommand::Remove { trip_id, stop } => remove(ctx, trip_id, stop),
        TripCommand::Delete { trip_id } => delete(ctx, trip_id),
    }
}

fn list(ctx: &Context, limit: usize) -> Result<Document, AxiError> {
    let limit = validate_limit(limit)?;
    let client = ctx.member_client("trip list")?;
    let trips = trips::list_trips(&client)?;
    let total = trips.len();
    let shown = total.min(limit);

    let mut doc = Document::new();
    doc.set("count", count_line(shown, total));
    doc.set_table("trips", trips_table(&trips[..shown]));
    doc.set_primary("trips");
    if total == 0 {
        doc.set_strings(
            "help",
            &["Run `chictrip-axi trip create --name \"<name>\" --start <date> --end <date> --location <key>` to start a new trip"],
        );
    } else {
        doc.set_strings(
            "help",
            &[
                "Run `chictrip-axi trip view <id>` for the day-by-day plan",
                "Run `chictrip-axi trip create --name \"<name>\" --start <date> --end <date> --location <key>` to start a new trip",
            ],
        );
    }
    Ok(doc)
}

#[allow(clippy::too_many_arguments)]
fn create(
    ctx: &Context,
    name: &str,
    start: &str,
    end: &str,
    traffic: &str,
    locations: &[String],
    duplicate: bool,
) -> Result<Document, AxiError> {
    if name.trim().is_empty() {
        return Err(AxiError::usage("--name is empty"));
    }
    let start = Date::parse(start)?;
    let end = Date::parse(end)?;
    let days = start.span_days(end);
    if days < 1 {
        return Err(AxiError::usage("--end is before --start"));
    }
    if days > MAX_TRIP_DAYS {
        return Err(AxiError::usage(format!(
            "a trip spans at most {MAX_TRIP_DAYS} days; this one spans {days}"
        )));
    }
    let traffic = TRAFFIC_MODES
        .iter()
        .find(|mode| mode.eq_ignore_ascii_case(traffic))
        .ok_or_else(|| {
            AxiError::usage(format!(
                "unknown --traffic '{traffic}'; valid modes: {}",
                TRAFFIC_MODES.join(",")
            ))
        })?;
    for key in locations {
        if key.split(',').count() != 3 || key.split(',').any(|p| p.trim().parse::<i64>().is_err()) {
            return Err(AxiError::usage(format!(
                "invalid --location '{key}'; expected country,city,area like 7,7,0"
            )));
        }
    }

    let client = ctx.member_client("trip create")?;
    let (start_api, end_api) = (start.to_api(), end.to_api());

    if !duplicate {
        let existing = trips::list_trips(&client)?.into_iter().find(|trip| {
            trip.name.as_deref() == Some(name)
                && trip.start_date.as_deref() == Some(start_api.as_str())
                && trip.end_date.as_deref() == Some(end_api.as_str())
        });
        if let Some(trip) = existing {
            let id = trip.id.clone().unwrap_or_default();
            let mut doc = Document::new();
            let mut detail = Document::new();
            detail.set("id", value(&trip.id));
            detail.set("name", value(&trip.name));
            detail.set("start", value(&trip.start_date));
            detail.set("end", value(&trip.end_date));
            detail.set("days", count(trip.total_day));
            detail.set("update_time", count(trip.update_time));
            doc.set_object("trip", detail);
            doc.set_primary("trip");
            doc.set("note", "already exists (no-op)");
            doc.set_strings("help", &created_help(&id));
            return Ok(doc);
        }
    }

    let label_id = trips::default_label_id(&client)?;
    let cover_id = trips::default_cover_id(&client)?;
    let mut form: Vec<(&str, String)> = vec![
        ("CoverMediaId", cover_id),
        ("Name", name.to_string()),
        ("StartDate", start_api.clone()),
        ("EndDate", end_api.clone()),
        ("TotalDay", days.to_string()),
        ("ViewMode", "DetailMode".to_string()),
        ("TravelScheduleUserLabelId", label_id),
        ("id", String::new()),
        ("TrafficType", (*traffic).to_string()),
        ("IsForceUpdateTsdRoute", "0".to_string()),
        ("updateTime", "0".to_string()),
    ];
    for key in locations {
        form.push(("LocationKey[]", key.trim().to_string()));
    }
    let data = client.post_form_zhtw("TravelSchedule/AddV2", &form)?;
    let created: CreatedTrip = serde_json::from_value(data)
        .map_err(|e| AxiError::internal(format!("chicTrip sent an unexpected shape: {e}")))?;
    let id = created.id.clone().unwrap_or_default();

    let mut detail = Document::new();
    detail.set("id", value(&created.id));
    detail.set(
        "name",
        created.name.clone().unwrap_or_else(|| name.to_string()),
    );
    detail.set("start", start_api);
    detail.set("end", end_api);
    detail.set("days", days);
    detail.set("update_time", count(created.update_time));

    let mut doc = Document::new();
    doc.set_object("trip", detail);
    doc.set_primary("trip");
    doc.set_strings("help", &created_help(&id));
    Ok(doc)
}

fn created_help(id: &str) -> Vec<String> {
    vec![
        format!(
            "Run `chictrip-axi trip add {id} --day 1 --poi <poi-id> --poi <poi-id>` to fill day 1"
        ),
        "Run `chictrip-axi poi search \"<keyword>\"` to find POI ids".to_string(),
    ]
}

fn view(ctx: &Context, trip_id: &str, day: Option<i64>, full: bool) -> Result<Document, AxiError> {
    let client = ctx.member_client("trip view")?;
    let update_time = trips::current_update_time(&client, trip_id)?;
    let detail = trips::trip_detail(&client, trip_id, update_time)?;
    let (mut doc, note_was_cut) = trip_document(&detail, day, full)?;
    let mut help = vec![
        format!("Run `chictrip-axi trip add {trip_id} --day <n> --poi <poi-id>` to add stops"),
        format!(
            "Run `chictrip-axi trip edit {trip_id} --stop <tsd_id> --stay <min>` to change a stop"
        ),
        format!(
            "Run `chictrip-axi trip view {trip_id} --full` for notes and legs and pinned times"
        ),
    ];
    if note_was_cut {
        help.push(format!(
            "Run `chictrip-axi trip note {trip_id}` for the whole trip note"
        ));
    }
    doc.set_strings("help", &help);
    Ok(doc)
}

fn trip_header(info: &crate::api::types::TripInfo) -> Document {
    let mut header = Document::new();
    header.set("id", value(&info.id));
    header.set("name", value(&info.name));
    header.set("start", value(&info.start_date));
    header.set("end", value(&info.end_date));
    header.set("days", count(info.total_day));
    header
}

/// Also reports whether the trip note was truncated, so `view` can offer
/// the command that prints the whole thing.
fn trip_document(
    detail: &TripDetail,
    day: Option<i64>,
    full: bool,
) -> Result<(Document, bool), AxiError> {
    let info = &detail.travel_schedule_info;
    let mut header = trip_header(info);
    header.set("permission", value(&info.permission));
    header.set("update_time", count(info.update_time));
    let mut note_was_cut = false;
    if let Value::String(note) = value(&info.note) {
        let (text, cut) = truncate(&note);
        note_was_cut = cut;
        header.set("note", text);
    }

    let days = select_days(&detail.day_list, day, info.total_day)?;
    let mut doc = Document::new();
    doc.set_object("trip", header);
    doc.set_table("stops", stops_table(&days, full, true));
    doc.set_primary("stops");
    Ok((doc, note_was_cut))
}

fn preview(
    ctx: &Context,
    trip_id: &str,
    day: Option<i64>,
    full: bool,
) -> Result<Document, AxiError> {
    let client = ctx.client()?;
    let detail = trips::trip_preview(&client, trip_id)?;
    let mut doc = preview_document(&detail, day, full)?;
    doc.set_strings(
        "help",
        &[
            "Run `chictrip-axi poi view <poi_id>` for a stop",
            "Run `chictrip-axi trip add <my-trip-id> --day <n> --poi <poi_id>` to copy a stop into my own trip",
        ],
    );
    Ok(doc)
}

/// A viewer cannot write a trip that is not theirs, so the owner's
/// `permission` and `update_time` and the per-stop `tsd_id` that only
/// `trip remove` takes would all be noise here.
fn preview_document(
    detail: &TripDetail,
    day: Option<i64>,
    full: bool,
) -> Result<Document, AxiError> {
    let info = &detail.travel_schedule_info;
    let days = select_days(&detail.day_list, day, info.total_day)?;
    let mut doc = Document::new();
    doc.set_object("trip", trip_header(info));
    doc.set_table("stops", stops_table(&days, full, false));
    doc.set_primary("stops");
    Ok(doc)
}

fn add(
    ctx: &Context,
    trip_id: &str,
    day: i64,
    pois: &[String],
    position: &str,
    allow_duplicate: bool,
) -> Result<Document, AxiError> {
    if day < 1 {
        return Err(AxiError::usage("--day starts at 1"));
    }
    let position = match position.to_ascii_lowercase().as_str() {
        "last" => Position::Last,
        "best" => Position::Best,
        other => {
            return Err(AxiError::usage(format!(
                "unknown --position '{other}'; valid values: last,best"
            )));
        }
    };

    let client = ctx.member_client("trip add")?;
    let mut update_time = trips::current_update_time(&client, trip_id)?;
    let detail = trips::trip_detail(&client, trip_id, update_time)?;
    let total_days = detail
        .travel_schedule_info
        .total_day
        .unwrap_or(detail.day_list.len() as i64);
    if day > total_days {
        return Err(AxiError::not_found(format!(
            "day {day} is outside this trip, which has {total_days} days"
        )));
    }

    let before = day_stops(&detail, day);
    let mut present: Vec<String> = before.iter().filter_map(|s| s.poi_id.clone()).collect();
    let known_tsd: Vec<String> = before.iter().filter_map(|s| s.id.clone()).collect();

    let mut added: Vec<(String, String)> = Vec::new();
    let mut skipped: Vec<(String, String)> = Vec::new();
    let mut failure: Option<AxiError> = None;

    for poi_id in pois {
        if !allow_duplicate && present.contains(poi_id) {
            skipped.push((poi_id.clone(), format!("already in day {day}")));
            continue;
        }
        let poi = match trips::poi_detail(&client, poi_id) {
            Ok(poi) => poi,
            Err(e) => {
                failure = Some(e);
                break;
            }
        };
        match trips::add_stop(&client, trip_id, day, &poi, position, update_time) {
            Ok(new_time) => {
                update_time = new_time;
                present.push(poi_id.clone());
                added.push((poi_id.clone(), poi.name.clone().unwrap_or_default()));
            }
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }

    if let Some(error) = failure {
        if added.is_empty() {
            return Err(error);
        }
        return Err(AxiError::new(
            error.code,
            format!(
                "added {} of {} stops before the request failed: {}",
                added.len(),
                pois.len(),
                error.message
            ),
        )
        .with_request_id(error.request_id)
        .with_help([format!(
            "Run `chictrip-axi trip view {trip_id} --day {day}` to see what landed"
        )]));
    }

    let mut table = Table::new(&["poi_id", "name", "tsd_id", "seq"], &[]);
    if !added.is_empty() {
        let after = trips::trip_detail(&client, trip_id, update_time)?;
        update_time = after
            .travel_schedule_info
            .update_time
            .unwrap_or(update_time);
        let stops = day_stops(&after, day);
        let mut used: Vec<String> = known_tsd.clone();
        for (poi_id, name) in &added {
            let found = stops.iter().enumerate().find(|(_, stop)| {
                stop.poi_id.as_deref() == Some(poi_id.as_str())
                    && !used.contains(stop.id.as_ref().unwrap_or(&String::new()))
            });
            let (tsd_id, seq) = match found {
                Some((index, stop)) => {
                    let id = stop.id.clone().unwrap_or_default();
                    used.push(id.clone());
                    (Value::from(id), Value::from(index as i64 + 1))
                }
                None => (Value::Null, Value::Null),
            };
            table.push(&[
                ("poi_id", Value::from(poi_id.as_str())),
                ("name", Value::from(name.as_str())),
                ("tsd_id", tsd_id),
                ("seq", seq),
            ]);
        }
    }

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    doc.set("day", day);
    doc.set_table("added", table);
    doc.set_primary("added");
    if !skipped.is_empty() {
        let mut table = Table::new(&["poi_id", "reason"], &[]);
        for (poi_id, reason) in &skipped {
            table.push(&[
                ("poi_id", Value::from(poi_id.as_str())),
                ("reason", Value::from(reason.as_str())),
            ]);
        }
        doc.set_table("skipped", table);
    }
    doc.set("update_time", update_time);
    doc.set_strings(
        "help",
        &[
            format!("Run `chictrip-axi trip view {trip_id} --day {day}` to see the day"),
            format!("Run `chictrip-axi trip remove {trip_id} --stop <tsd_id>` to undo"),
        ],
    );
    Ok(doc)
}

fn remove(ctx: &Context, trip_id: &str, stops: &[String]) -> Result<Document, AxiError> {
    let client = ctx.member_client("trip remove")?;
    let mut update_time = trips::current_update_time(&client, trip_id)?;
    let detail = trips::trip_detail(&client, trip_id, update_time)?;

    let mut removed = Table::new(&["tsd_id", "name", "day"], &[]);
    let mut skipped = Table::new(&["tsd_id", "reason"], &[]);
    for tsd_id in stops {
        let found = detail.day_list.iter().find_map(|day| {
            day.tsd_list
                .iter()
                .find(|stop| stop.id.as_deref() == Some(tsd_id.as_str()))
                .map(|stop| (day.day.or(stop.day).unwrap_or(0), stop.clone()))
        });
        let Some((day, stop)) = found else {
            skipped.push(&[
                ("tsd_id", Value::from(tsd_id.as_str())),
                ("reason", Value::from("not in trip (no-op)")),
            ]);
            continue;
        };
        update_time = trips::remove_stop(&client, trip_id, day, tsd_id, update_time)?;
        removed.push(&[
            ("tsd_id", Value::from(tsd_id.as_str())),
            ("name", value(&stop.name)),
            ("day", Value::from(day)),
        ]);
    }

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    doc.set_table("removed", removed);
    doc.set_primary("removed");
    if !skipped.is_empty() {
        doc.set_table("skipped", skipped);
    }
    doc.set("update_time", update_time);
    doc.set_strings(
        "help",
        &[format!("Run `chictrip-axi trip view {trip_id}` to confirm")],
    );
    Ok(doc)
}

fn delete(ctx: &Context, trip_id: &str) -> Result<Document, AxiError> {
    let client = ctx.member_client("trip delete")?;
    let mut doc = Document::new();
    match trips::delete_trip(&client, trip_id) {
        Ok(()) => doc.set("deleted", trip_id),
        Err(e) if is_absent(&e) => {
            doc.set("deleted", trip_id);
            doc.set("note", "already absent (no-op)");
        }
        Err(e) => return Err(e),
    }
    doc.set_strings(
        "help",
        &["Run `chictrip-axi trip list` to see what is left"],
    );
    Ok(doc)
}

fn is_absent(error: &AxiError) -> bool {
    error.code == ErrorCode::NotFound
}

fn day_stops(detail: &TripDetail, day: i64) -> Vec<crate::api::types::Tsd> {
    detail
        .day_list
        .iter()
        .filter(|d: &&Day| d.day == Some(day))
        .flat_map(|d| d.tsd_list.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::TripInfo;

    #[test]
    fn an_empty_trip_still_prints_a_stops_header() {
        let detail = TripDetail {
            travel_schedule_info: TripInfo {
                id: Some("t1".into()),
                total_day: Some(2),
                ..TripInfo::default()
            },
            day_list: vec![],
        };
        let (doc, _) = trip_document(&detail, None, false).unwrap();
        assert!(crate::output::render(&doc, false).contains("stops[0]:"));
    }

    #[test]
    fn a_preview_hides_the_owner_fields_and_the_tsd_id() {
        use crate::api::types::Tsd;

        let detail = TripDetail {
            travel_schedule_info: TripInfo {
                id: Some("t1".into()),
                name: Some("Shared".into()),
                total_day: Some(1),
                permission: Some("Owner".into()),
                update_time: Some(1_789_710_463),
                ..TripInfo::default()
            },
            day_list: vec![Day {
                day: Some(1),
                date: Some("2026/10/01".into()),
                tsd_list: vec![Tsd {
                    id: Some("tsd-1".into()),
                    name: Some("Kaminarimon".into()),
                    ..Tsd::default()
                }],
                ..Day::default()
            }],
        };
        let rendered =
            crate::output::render(&preview_document(&detail, None, false).unwrap(), false);
        assert!(!rendered.contains("permission"), "{rendered}");
        assert!(!rendered.contains("update_time"), "{rendered}");
        assert!(!rendered.contains("tsd_id"), "{rendered}");
        assert!(
            rendered.contains("stops[1]{day,seq,arrive,stay_min,name,type,city,poi_id}:"),
            "{rendered}"
        );
    }
}
