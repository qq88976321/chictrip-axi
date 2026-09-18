//! `trip list`, `create`, `view`, `add`, `remove`, and `delete`.

use super::{
    MAX_LEG_MIN, MAX_STAY_MIN, count, count_line, select_days, stops_table, trips_table,
    validate_limit, validate_minutes, value,
};
use crate::api::trips::{self, Position, TrafficMode};
use crate::api::types::{Category, CreatedTrip, Day, EditInfo, Fare, TripDetail, Tsd};
use crate::cli::{Context, TripCommand};
use crate::datetime::{Date, parse_clock};
use crate::error::{AxiError, ErrorCode};
use crate::output::{Document, Table, truncate};
use serde_json::Value;

const TRAFFIC_MODES: [&str; 5] = ["Custom", "Transit", "Driving", "Walk", "PublicTransport"];
/// chicTrip's name for the slot in front of a day's first stop.
const START_SLOT: &str = "start";
/// The `categoryList` rows that make a stop a flight row.
const TSD_CATEGORY: &str = "TsdCategory";
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
            after,
            allow_duplicate,
        } => add(
            ctx,
            trip_id,
            *day,
            poi,
            position,
            after.as_deref(),
            *allow_duplicate,
        ),
        TripCommand::Edit {
            trip_id,
            stop,
            stay,
            arrive,
            depart,
            category,
            name,
        } => edit(
            ctx,
            trip_id,
            stop,
            &EditFlags {
                stay: *stay,
                arrive: arrive.as_deref(),
                depart: depart.as_deref(),
                category: category.as_deref(),
                name: name.as_deref(),
            },
        ),
        TripCommand::Note {
            trip_id,
            stop,
            set,
            clear,
        } => note(ctx, trip_id, stop.as_deref(), set.as_deref(), *clear),
        TripCommand::Leg {
            trip_id,
            stop,
            mode,
            route,
            custom,
            flight,
            note,
        } => leg(
            ctx,
            trip_id,
            stop,
            &LegFlags {
                mode: mode.as_deref(),
                route: route.as_deref(),
                custom: *custom,
                flight: *flight,
                note: note.as_deref(),
            },
        ),
        TripCommand::Traffic {
            trip_id,
            day,
            mode,
            recompute,
        } => traffic(ctx, trip_id, *day, mode, *recompute),
        TripCommand::Move {
            trip_id,
            stop,
            after,
            position,
            day,
        } => move_stop(
            ctx,
            trip_id,
            stop,
            after.as_deref(),
            position.as_deref(),
            *day,
        ),
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

/// Where the batch goes, before the day is read and the slot is known.
enum Target {
    Last,
    Best,
    First,
    After(String),
}

#[allow(clippy::too_many_arguments)]
fn add(
    ctx: &Context,
    trip_id: &str,
    day: i64,
    pois: &[String],
    position: &str,
    after: Option<&str>,
    allow_duplicate: bool,
) -> Result<Document, AxiError> {
    if day < 1 {
        return Err(AxiError::usage("--day starts at 1"));
    }
    let target = match after {
        Some(anchor) => Target::After(anchor.to_string()),
        None => match position.to_ascii_lowercase().as_str() {
            "last" => Target::Last,
            "first" => Target::First,
            "best" => Target::Best,
            other => {
                return Err(AxiError::usage(format!(
                    "unknown --position '{other}'; valid values: last,first,best"
                )));
            }
        },
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
    let (head_slot, rest_slot) = slots_for(&before, &target, trip_id, day)?;

    let mut added: Vec<(String, String, String)> = Vec::new();
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
        let position = if added.is_empty() {
            &head_slot
        } else {
            &rest_slot
        };
        match trips::add_stop(&client, trip_id, day, &poi, position, update_time) {
            Ok((tsd_id, new_time)) => {
                update_time = new_time;
                present.push(poi_id.clone());
                added.push((poi_id.clone(), poi.name.clone().unwrap_or_default(), tsd_id));
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
        let after: TripDetail = trips::trip_detail(&client, trip_id, update_time)?;
        update_time = after
            .travel_schedule_info
            .update_time
            .unwrap_or(update_time);
        let stops = day_stops(&after, day);
        for (poi_id, name, tsd_id) in &added {
            let seq = stops
                .iter()
                .position(|stop| stop.id.as_deref() == Some(tsd_id.as_str()))
                .map(|index| Value::from(index as i64 + 1))
                .unwrap_or(Value::Null);
            table.push(&[
                ("poi_id", Value::from(poi_id.as_str())),
                ("name", Value::from(name.as_str())),
                ("tsd_id", Value::from(tsd_id.as_str())),
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

/// The slot the first insert of a batch uses and the slot every later one
/// uses. They differ only for `first`: the day's old first stop has no slot
/// of its own until something is put in front of it, and from then on it is
/// the anchor that keeps `--poi A --poi B` in the order it was given.
fn slots_for(
    stops: &[crate::api::types::Tsd],
    target: &Target,
    trip_id: &str,
    day: i64,
) -> Result<(Position, Position), AxiError> {
    let id_at = |index: usize| stops.get(index).and_then(|s| s.id.clone());
    let same = |slot: Option<String>| {
        let slot = slot.map_or(Position::Last, Position::Slot);
        (slot.clone(), slot)
    };
    match target {
        Target::Last => Ok((Position::Last, Position::Last)),
        Target::Best => Ok((Position::Best, Position::Best)),
        Target::First => Ok(match id_at(0) {
            Some(first) => (
                Position::Slot(START_SLOT.to_string()),
                Position::Slot(first),
            ),
            None => (Position::Last, Position::Last),
        }),
        Target::After(anchor) => {
            let index = stops
                .iter()
                .position(|s| s.id.as_deref() == Some(anchor.as_str()))
                .ok_or_else(|| {
                    AxiError::not_found(format!("stop {anchor} is not in day {day} of this trip"))
                        .with_help([format!(
                            "Run `chictrip-axi trip view {trip_id} --day {day}` to list that day"
                        )])
                })?;
            Ok(same(id_at(index + 1)))
        }
    }
}

/// The `trip edit` flags as clap parsed them, before validation.
struct EditFlags<'a> {
    stay: Option<i64>,
    arrive: Option<&'a str>,
    depart: Option<&'a str>,
    category: Option<&'a str>,
    name: Option<&'a str>,
}

/// What the caller asked of one time field.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TimeFlag {
    Keep,
    Auto,
    At(String),
}

/// The validated request, with the category still an icon token because the
/// id it maps to is only known once the live sheet is read.
struct EditRequest {
    stay: Option<i64>,
    arrive: TimeFlag,
    depart: TimeFlag,
    category: Option<String>,
    name: Option<String>,
}

fn parse_time_flag(flag: &str, text: Option<&str>) -> Result<TimeFlag, AxiError> {
    match text {
        None => Ok(TimeFlag::Keep),
        Some(text) if text.eq_ignore_ascii_case("auto") => Ok(TimeFlag::Auto),
        Some(text) => Ok(TimeFlag::At(parse_clock(flag, text)?)),
    }
}

fn validate_edit(flags: &EditFlags) -> Result<EditRequest, AxiError> {
    if flags.stay.is_none()
        && flags.arrive.is_none()
        && flags.depart.is_none()
        && flags.category.is_none()
        && flags.name.is_none()
    {
        return Err(AxiError::usage(
            "nothing to change; pass at least one of --stay --arrive --depart --category --name",
        ));
    }
    let stay = flags
        .stay
        .map(|value| validate_minutes("--stay", value, MAX_STAY_MIN))
        .transpose()?;
    let name = match flags.name {
        Some(name) if name.trim().is_empty() => return Err(AxiError::usage("--name is empty")),
        other => other.map(str::to_string),
    };
    let category = match flags.category {
        Some(token) if token.trim().is_empty() => {
            return Err(AxiError::usage("--category is empty"));
        }
        other => other.map(|token| token.trim().to_string()),
    };
    Ok(EditRequest {
        stay,
        arrive: parse_time_flag("--arrive", flags.arrive)?,
        depart: parse_time_flag("--depart", flags.depart)?,
        category,
        name,
    })
}

fn resolve_category(info: &EditInfo, token: &str) -> Result<Category, AxiError> {
    info.category_list
        .iter()
        .find(|row| {
            row.icon
                .as_deref()
                .is_some_and(|icon| icon.eq_ignore_ascii_case(token))
        })
        .cloned()
        .ok_or_else(|| {
            let icons: Vec<&str> = info
                .category_list
                .iter()
                .filter_map(|row| row.icon.as_deref())
                .collect();
            AxiError::usage(format!(
                "unknown --category '{token}'; valid categories: {}",
                icons.join(",")
            ))
        })
}

fn pinned(in_use: bool, time: &Option<String>) -> Option<String> {
    in_use.then(|| time.clone()).flatten()
}

fn merged_time(flag: &TimeFlag, current: &Option<String>) -> Option<String> {
    match flag {
        TimeFlag::Keep => current.clone(),
        TimeFlag::Auto => None,
        TimeFlag::At(time) => Some(time.clone()),
    }
}

/// Folds the request into the sheet chicTrip answered with, and names every
/// field that came out different. An empty list is the no-op.
fn merge_edit(
    info: &EditInfo,
    request: &EditRequest,
    category: Option<&Category>,
) -> (trips::StopEdit, Vec<&'static str>) {
    let current_name = info.name.clone().unwrap_or_default();
    let current_category = info.poi_classification_id.clone().unwrap_or_default();
    let current_stay = info.stay_time.unwrap_or(0);
    let current_arrival = pinned(info.is_use_custom_arrival_time, &info.custom_arrival_time);
    let current_departure = pinned(
        info.is_use_custom_departure_time,
        &info.custom_departure_time,
    );

    let edit = trips::StopEdit {
        name: request.name.clone().unwrap_or_else(|| current_name.clone()),
        category_id: category
            .and_then(|row| row.id.clone())
            .unwrap_or_else(|| current_category.clone()),
        stay_time: request.stay.unwrap_or(current_stay),
        arrival: merged_time(&request.arrive, &current_arrival),
        departure: merged_time(&request.depart, &current_departure),
    };

    let mut changed = Vec::new();
    if edit.name != current_name {
        changed.push("name");
    }
    if edit.category_id != current_category {
        changed.push("category");
    }
    if edit.stay_time != current_stay {
        changed.push("stay_min");
    }
    if edit.arrival != current_arrival {
        changed.push("arrive");
    }
    if edit.departure != current_departure {
        changed.push("depart");
    }
    (edit, changed)
}

/// The stop as chicTrip now holds it, from the sheet alone.
fn stop_object(info: &EditInfo, tsd_id: &str) -> Document {
    let is_flight = info
        .category_list
        .iter()
        .find(|row| row.icon == info.category_icon)
        .and_then(|row| row.category_type.as_deref())
        .is_some_and(|kind| kind == TSD_CATEGORY);

    let mut stop = Document::new();
    stop.set("tsd_id", tsd_id);
    stop.set("name", value(&info.name));
    stop.set("category", value(&info.category_icon));
    stop.set("type", if is_flight { "flight" } else { "basic" });
    let arrive_pinned = pinned(info.is_use_custom_arrival_time, &info.custom_arrival_time);
    stop.set(
        "arrive",
        value(&arrive_pinned.clone().or_else(|| info.arrival_time.clone())),
    );
    if arrive_pinned.is_some() {
        stop.set("arrive_custom", true);
    }
    let depart_pinned = pinned(
        info.is_use_custom_departure_time,
        &info.custom_departure_time,
    );
    stop.set(
        "depart",
        value(
            &depart_pinned
                .clone()
                .or_else(|| info.departure_time.clone()),
        ),
    );
    if depart_pinned.is_some() {
        stop.set("depart_custom", true);
    }
    stop.set("stay_min", count(info.stay_time));
    stop
}

fn edit(
    ctx: &Context,
    trip_id: &str,
    tsd_id: &str,
    flags: &EditFlags,
) -> Result<Document, AxiError> {
    let request = validate_edit(flags)?;

    let client = ctx.member_client("trip edit")?;
    let update_time = trips::current_update_time(&client, trip_id)?;
    let info = trips::edit_info(&client, trip_id, tsd_id, update_time).map_err(|e| {
        if e.code == ErrorCode::NotFound {
            return e.with_help([format!(
                "Run `chictrip-axi trip view {trip_id}` to list the stops and their tsd_id"
            )]);
        }
        e
    })?;
    let category = request
        .category
        .as_deref()
        .map(|token| resolve_category(&info, token))
        .transpose()?;
    let (edit, changed) = merge_edit(&info, &request, category.as_ref());

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    if changed.is_empty() {
        doc.set_object("stop", stop_object(&info, tsd_id));
        doc.set_primary("stop");
        doc.set_strings("changed", &[] as &[String]);
        doc.set("note", "already as requested (no-op)");
        doc.set_strings("help", &edited_help(trip_id, tsd_id));
        return Ok(doc);
    }

    let update_time = trips::update_stop(&client, trip_id, tsd_id, &edit, update_time)?;
    let after = trips::edit_info(&client, trip_id, tsd_id, update_time)?;
    doc.set_object("stop", stop_object(&after, tsd_id));
    doc.set_primary("stop");
    doc.set_strings("changed", &changed);
    doc.set("update_time", update_time);
    doc.set_strings("help", &edited_help(trip_id, tsd_id));
    Ok(doc)
}

fn edited_help(trip_id: &str, tsd_id: &str) -> Vec<String> {
    vec![
        format!("Run `chictrip-axi trip view {trip_id} --full` to see every stop with its times"),
        format!(
            "Run `chictrip-axi trip note {trip_id} --stop {tsd_id} --set \"<text>\"` to attach a note"
        ),
    ]
}

/// Which day a stop sits in, where in that day, and the row itself.
fn locate_stop(detail: &TripDetail, tsd_id: &str) -> Option<(i64, usize, Tsd)> {
    detail.day_list.iter().find_map(|day| {
        day.tsd_list
            .iter()
            .enumerate()
            .find(|(_, stop)| stop.id.as_deref() == Some(tsd_id))
            .map(|(index, stop)| (day.day.or(stop.day).unwrap_or(0), index, stop.clone()))
    })
}

fn note(
    ctx: &Context,
    trip_id: &str,
    tsd_id: Option<&str>,
    set: Option<&str>,
    clear: bool,
) -> Result<Document, AxiError> {
    if let Some(text) = set {
        if text.trim().is_empty() {
            return Err(AxiError::usage(
                "--set is empty; pass --clear to remove the note",
            ));
        }
    }

    let client = ctx.member_client("trip note")?;
    let update_time = trips::current_update_time(&client, trip_id)?;
    let detail = trips::trip_detail(&client, trip_id, update_time)?;

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    let current = match tsd_id {
        Some(tsd_id) => {
            let (_, _, stop) = locate_stop(&detail, tsd_id).ok_or_else(|| {
                AxiError::not_found(format!("stop {tsd_id} is not in trip {trip_id}")).with_help([
                    format!(
                        "Run `chictrip-axi trip view {trip_id}` to list the stops and their tsd_id"
                    ),
                ])
            })?;
            doc.set("stop_id", tsd_id);
            doc.set("name", value(&stop.name));
            stop.note.clone().unwrap_or_default()
        }
        None => detail.travel_schedule_info.note.clone().unwrap_or_default(),
    };

    let wanted = match (set, clear) {
        (None, false) => {
            doc.set("note", current.as_str());
            doc.set_strings("help", &note_help(trip_id, tsd_id));
            return Ok(doc);
        }
        (Some(text), _) => text.to_string(),
        (None, true) => String::new(),
    };

    doc.set("note", wanted.as_str());
    if wanted == current {
        doc.set("status", "unchanged (no-op)");
        doc.set_strings("help", &note_help(trip_id, tsd_id));
        return Ok(doc);
    }

    let update_time = match tsd_id {
        Some(tsd_id) => trips::set_stop_note(&client, trip_id, tsd_id, &wanted, update_time)?,
        None => trips::set_trip_note(&client, trip_id, &wanted, update_time)?,
    };
    doc.set("status", if wanted.is_empty() { "cleared" } else { "set" });
    doc.set("update_time", update_time);
    doc.set_strings("help", &note_help(trip_id, tsd_id));
    Ok(doc)
}

fn note_help(trip_id: &str, tsd_id: Option<&str>) -> Vec<String> {
    match tsd_id {
        Some(tsd_id) => vec![format!(
            "Run `chictrip-axi trip note {trip_id} --stop {tsd_id} --set \"<text>\"` to change it"
        )],
        None => vec![format!(
            "Run `chictrip-axi trip note {trip_id} --set \"<text>\"` to change it"
        )],
    }
}

struct LegFlags<'a> {
    mode: Option<&'a str>,
    route: Option<&'a str>,
    custom: Option<i64>,
    flight: Option<i64>,
    note: Option<&'a str>,
}

fn routable_mode(token: &str) -> Result<TrafficMode, AxiError> {
    TrafficMode::from_token(token)
        .filter(|mode| mode.is_routable())
        .ok_or_else(|| {
            let tokens: Vec<&str> = TrafficMode::ROUTABLE.iter().map(|m| m.token()).collect();
            AxiError::usage(format!(
                "unknown --mode '{token}'; valid modes: {}",
                tokens.join(",")
            ))
        })
}

/// Metres to kilometres with one decimal, the way a map app shows them.
fn kilometres(metres: Option<f64>) -> Value {
    metres
        .map(|m| Value::from((m / 100.0).round() / 10.0))
        .unwrap_or(Value::Null)
}

fn fare_text(fare: &Option<Fare>) -> Value {
    let Some(fare) = fare else {
        return Value::Null;
    };
    let Some(amount) = fare.value else {
        return Value::Null;
    };
    let currency = fare.currency.clone().unwrap_or_default();
    let amount = if amount.fract() == 0.0 {
        format!("{}", amount as i64)
    } else {
        format!("{amount}")
    };
    Value::from(format!("{currency} {amount}").trim().to_string())
}

/// The stop the leg arrives at, and where it comes from.
fn leg_stop_object(stops: &[Tsd], index: usize, day: i64) -> Document {
    let stop = &stops[index];
    let mut object = Document::new();
    object.set("tsd_id", value(&stop.id));
    object.set("name", value(&stop.name));
    object.set("day", day);
    if let Some(previous) = index.checked_sub(1).and_then(|i| stops.get(i)) {
        object.set("from", value(&previous.name));
    }
    object.set("traffic", value(&stop.arrival_traffic_type));
    object.set("traffic_min", count(stop.arrival_traffic_time));
    object
}

fn leg(ctx: &Context, trip_id: &str, tsd_id: &str, flags: &LegFlags) -> Result<Document, AxiError> {
    let wanted_mode = flags.mode.map(routable_mode).transpose()?;
    let duration = match (flags.custom, flags.flight) {
        (Some(minutes), None) => Some((validate_minutes("--custom", minutes, MAX_LEG_MIN)?, false)),
        (None, Some(minutes)) => Some((validate_minutes("--flight", minutes, MAX_LEG_MIN)?, true)),
        _ => None,
    };

    let client = ctx.member_client("trip leg")?;
    let update_time = trips::current_update_time(&client, trip_id)?;
    let detail = trips::trip_detail(&client, trip_id, update_time)?;
    let (day, index, row) = locate_stop(&detail, tsd_id).ok_or_else(|| {
        AxiError::not_found(format!("stop {tsd_id} is not in trip {trip_id}")).with_help([format!(
            "Run `chictrip-axi trip view {trip_id}` to list the stops and their tsd_id"
        )])
    })?;
    let stops = day_stops(&detail, day);

    let Some(route_id) = row.tsd_route_detail_id.clone().filter(|_| index > 0) else {
        let name = row.name.clone().unwrap_or_default();
        let error = AxiError::not_found(format!(
            "{name} is the first stop of day {day} and has no leg into it"
        ));
        let next = stops.get(index + 1).and_then(|s| s.id.clone());
        return Err(match next {
            Some(next) => error.with_help([format!(
                "Run `chictrip-axi trip leg {trip_id} --stop {next}` for the leg into the next stop"
            )]),
            None => error,
        });
    };

    if let Some(route) = flags.route {
        let current = row
            .arrival_traffic_type
            .as_deref()
            .and_then(TrafficMode::from_api)
            .filter(|mode| mode.is_routable());
        if let Some(mode) = current {
            let list = trips::route_list(&client, trip_id, &route_id, mode, update_time)?;
            let already = list
                .tsd_route_list
                .iter()
                .chain(list.tsd_route_transit_list.iter())
                .any(|option| {
                    option.poi_route_detail_id.as_deref() == Some(route) && option.is_selected
                });
            if already {
                return Ok(leg_no_op(
                    trip_id,
                    &stops,
                    index,
                    day,
                    "already selected (no-op)",
                ));
            }
        }
        let update_time = trips::set_route(&client, trip_id, &route_id, route, update_time)?;
        return leg_after_write(&client, trip_id, tsd_id, update_time);
    }

    if let Some((minutes, is_flight)) = duration {
        let mode = if is_flight {
            TrafficMode::Flight
        } else {
            TrafficMode::Custom
        };
        let note = flags.note.unwrap_or_default();
        let list = trips::route_list(&client, trip_id, &route_id, mode, update_time)?;
        let stored = list.tsd_custom_route.unwrap_or_default();
        let already = row.arrival_traffic_type.as_deref() == Some(mode.api())
            && stored.duration == Some(minutes)
            && stored.note.unwrap_or_default() == note;
        if already {
            return Ok(leg_no_op(
                trip_id,
                &stops,
                index,
                day,
                "already set (no-op)",
            ));
        }
        let update_time = trips::set_custom_route(
            &client,
            trip_id,
            &route_id,
            minutes,
            note,
            is_flight,
            update_time,
        )?;
        return leg_after_write(&client, trip_id, tsd_id, update_time);
    }

    let mode = wanted_mode
        .or_else(|| {
            row.arrival_traffic_type
                .as_deref()
                .and_then(TrafficMode::from_api)
                .filter(|mode| mode.is_routable())
        })
        .unwrap_or(TrafficMode::Driving);
    let list = trips::route_list(&client, trip_id, &route_id, mode, update_time)?;

    let mut table = Table::new(
        &["route_id", "minutes", "km", "summary", "fare", "selected"],
        &[],
    );
    for option in list
        .tsd_route_list
        .iter()
        .chain(list.tsd_route_transit_list.iter())
    {
        table.push(&[
            ("route_id", value(&option.poi_route_detail_id)),
            ("minutes", count(option.duration)),
            ("km", kilometres(option.distance)),
            ("summary", value(&option.summary)),
            ("fare", fare_text(&option.fare)),
            ("selected", Value::from(option.is_selected)),
        ]);
    }

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    doc.set_object("stop", leg_stop_object(&stops, index, day));
    doc.set("mode", mode.api());
    doc.set("count", table.len() as i64);
    let empty = table.is_empty();
    doc.set_table("routes", table);
    doc.set_primary("routes");
    if let Some(custom) = list.tsd_custom_route.filter(|c| c.duration.is_some()) {
        let mut object = Document::new();
        object.set("minutes", count(custom.duration));
        object.set("note", value(&custom.note));
        object.set("traffic", value(&custom.traffic_type));
        doc.set_object("custom", object);
    }
    let mut help = vec![
        format!(
            "Run `chictrip-axi trip leg {trip_id} --stop {tsd_id} --route <route_id>` to pick one"
        ),
        format!(
            "Run `chictrip-axi trip leg {trip_id} --stop {tsd_id} --mode <driving|transit|walking|scooter>` for another mode"
        ),
        format!(
            "Run `chictrip-axi trip leg {trip_id} --stop {tsd_id} --custom <min> --note \"<text>\"` for a leg chicTrip cannot route"
        ),
    ];
    if empty {
        help.remove(0);
    }
    doc.set_strings("help", &help);
    Ok(doc)
}

fn leg_no_op(trip_id: &str, stops: &[Tsd], index: usize, day: i64, note: &str) -> Document {
    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    doc.set_object("stop", leg_stop_object(stops, index, day));
    doc.set_primary("stop");
    doc.set("note", note);
    doc
}

fn leg_after_write(
    client: &crate::api::Client,
    trip_id: &str,
    tsd_id: &str,
    update_time: i64,
) -> Result<Document, AxiError> {
    let detail = trips::trip_detail(client, trip_id, update_time)?;
    let (day, index, _) = locate_stop(&detail, tsd_id)
        .ok_or_else(|| AxiError::not_found(format!("stop {tsd_id} is not in trip {trip_id}")))?;
    let stops = day_stops(&detail, day);

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    doc.set_object("stop", leg_stop_object(&stops, index, day));
    doc.set_primary("stop");
    doc.set("update_time", update_time);
    doc.set_strings(
        "help",
        &[
            format!("Run `chictrip-axi trip view {trip_id} --day {day} --full` to check the day"),
            format!(
                "Run `chictrip-axi trip leg {trip_id} --stop {tsd_id}` to list the routes again"
            ),
        ],
    );
    Ok(doc)
}

/// The modes a whole day can default to: the routable ones plus `custom`,
/// which means "I will fill the legs in myself".
fn day_mode(token: &str) -> Result<TrafficMode, AxiError> {
    TrafficMode::from_token(token)
        .filter(|mode| mode.is_routable() || *mode == TrafficMode::Custom)
        .ok_or_else(|| {
            let mut tokens = vec![TrafficMode::Custom.token()];
            tokens.extend(TrafficMode::ROUTABLE.iter().map(|m| m.token()));
            AxiError::usage(format!(
                "unknown --mode '{token}'; valid modes: {}",
                tokens.join(",")
            ))
        })
}

fn traffic(
    ctx: &Context,
    trip_id: &str,
    day: i64,
    mode: &str,
    recompute: bool,
) -> Result<Document, AxiError> {
    if day < 1 {
        return Err(AxiError::usage("--day starts at 1"));
    }
    let mode = day_mode(mode)?;

    let client = ctx.member_client("trip traffic")?;
    let update_time = trips::current_update_time(&client, trip_id)?;
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
    let current = detail.day_list.iter().find(|d| d.day == Some(day)).cloned();

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    doc.set("day", day);

    let already = !recompute
        && current
            .as_ref()
            .and_then(|d| d.traffic_type.as_deref())
            .is_some_and(|kind| kind == mode.api());
    let (shown, update_time) = if already {
        (current.unwrap_or_default(), update_time)
    } else {
        let answer = trips::set_day_traffic(&client, trip_id, day, mode, recompute, update_time)?;
        let new_time = answer.travel_schedule_update_time.unwrap_or(update_time);
        (answer.day_data, new_time)
    };

    doc.set("traffic", value(&shown.traffic_type));
    let mut table = stops_table(std::slice::from_ref(&shown), false, true);
    table.select(&[
        "seq".into(),
        "arrive".into(),
        "stay_min".into(),
        "name".into(),
        "tsd_id".into(),
        "traffic".into(),
        "traffic_min".into(),
    ])?;
    doc.set_table("stops", table);
    doc.set_primary("stops");
    if already {
        doc.set("note", format!("already {} (no-op)", mode.api()));
    } else {
        doc.set("update_time", update_time);
    }
    doc.set_strings(
        "help",
        &[
            format!("Run `chictrip-axi trip leg {trip_id} --stop <tsd_id>` to change one leg"),
            format!("Run `chictrip-axi trip view {trip_id} --day {day} --full` for the whole day"),
        ],
    );
    Ok(doc)
}

/// Where a moved stop should end up inside its target day.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MoveTarget {
    First,
    Last,
    After(String),
}

/// The target day's ids in their new order, the moved stop included.
fn moved_order(ids: &[String], stop: &str, target: &MoveTarget) -> Vec<String> {
    let mut order: Vec<String> = ids.iter().filter(|id| *id != stop).cloned().collect();
    let at = match target {
        MoveTarget::First => 0,
        MoveTarget::Last => order.len(),
        MoveTarget::After(anchor) => order
            .iter()
            .position(|id| id == anchor)
            .map_or(order.len(), |index| index + 1),
    };
    order.insert(at, stop.to_string());
    order
}

fn move_stop(
    ctx: &Context,
    trip_id: &str,
    tsd_id: &str,
    after: Option<&str>,
    position: Option<&str>,
    day: Option<i64>,
) -> Result<Document, AxiError> {
    let target = match (after, position) {
        (Some(anchor), _) if anchor == tsd_id => {
            return Err(AxiError::usage("--after names the stop itself"));
        }
        (Some(anchor), _) => MoveTarget::After(anchor.to_string()),
        (None, Some(where_to)) => match where_to.to_ascii_lowercase().as_str() {
            "first" => MoveTarget::First,
            "last" => MoveTarget::Last,
            other => {
                return Err(AxiError::usage(format!(
                    "unknown --position '{other}'; valid values: first,last"
                )));
            }
        },
        (None, None) => {
            return Err(AxiError::usage(
                "pass --after <tsd-id> or --position first|last",
            ));
        }
    };
    if day.is_some_and(|day| day < 1) {
        return Err(AxiError::usage("--day starts at 1"));
    }

    let client = ctx.member_client("trip move")?;
    let update_time = trips::current_update_time(&client, trip_id)?;
    let detail = trips::trip_detail(&client, trip_id, update_time)?;
    let (from_day, _, row) = locate_stop(&detail, tsd_id).ok_or_else(|| {
        AxiError::not_found(format!("stop {tsd_id} is not in trip {trip_id}")).with_help([format!(
            "Run `chictrip-axi trip view {trip_id}` to list the stops and their tsd_id"
        )])
    })?;
    let to_day = day.unwrap_or(from_day);
    let total_days = detail
        .travel_schedule_info
        .total_day
        .unwrap_or(detail.day_list.len() as i64);
    if to_day > total_days {
        return Err(AxiError::not_found(format!(
            "day {to_day} is outside this trip, which has {total_days} days"
        )));
    }

    let current: Vec<String> = day_stops(&detail, to_day)
        .iter()
        .filter_map(|s| s.id.clone())
        .collect();
    if let MoveTarget::After(anchor) = &target {
        if !current.iter().any(|id| id == anchor) {
            return Err(AxiError::not_found(format!(
                "stop {anchor} is not in day {to_day} of this trip"
            ))
            .with_help([format!(
                "Run `chictrip-axi trip view {trip_id} --day {to_day}` to list that day"
            )]));
        }
    }
    let order = moved_order(&current, tsd_id, &target);

    if to_day == from_day && order == current {
        let mut doc = Document::new();
        doc.set("trip_id", trip_id);
        doc.set_object(
            "moved",
            moved_object(&row, from_day, to_day, &order, tsd_id),
        );
        doc.set_primary("moved");
        doc.set("note", "already in place (no-op)");
        return Ok(doc);
    }

    let update_time = trips::sort_day(
        &client,
        trip_id,
        from_day,
        to_day,
        tsd_id,
        &order,
        update_time,
    )?;
    let after_move = trips::trip_detail(&client, trip_id, update_time)?;
    let stops = day_stops(&after_move, to_day);
    let ids: Vec<String> = stops.iter().filter_map(|s| s.id.clone()).collect();

    let mut doc = Document::new();
    doc.set("trip_id", trip_id);
    doc.set_object("moved", moved_object(&row, from_day, to_day, &ids, tsd_id));
    let day_list = vec![Day {
        day: Some(to_day),
        tsd_list: stops,
        ..Day::default()
    }];
    let mut table = stops_table(&day_list, false, true);
    table.select(&[
        "seq".into(),
        "arrive".into(),
        "stay_min".into(),
        "name".into(),
        "tsd_id".into(),
    ])?;
    doc.set_table("stops", table);
    doc.set_primary("stops");
    doc.set("update_time", update_time);
    doc.set_strings(
        "help",
        &[format!(
            "Run `chictrip-axi trip view {trip_id} --day {to_day} --full` to check the legs after the move"
        )],
    );
    Ok(doc)
}

fn moved_object(row: &Tsd, from_day: i64, to_day: i64, order: &[String], tsd_id: &str) -> Document {
    let mut object = Document::new();
    object.set("tsd_id", tsd_id);
    object.set("name", value(&row.name));
    object.set("from_day", from_day);
    object.set("day", to_day);
    if let Some(index) = order.iter().position(|id| id == tsd_id) {
        object.set("seq", index as i64 + 1);
    }
    object
}

fn remove(ctx: &Context, trip_id: &str, stops: &[String]) -> Result<Document, AxiError> {
    let client = ctx.member_client("trip remove")?;
    let mut update_time = trips::current_update_time(&client, trip_id)?;
    let detail = trips::trip_detail(&client, trip_id, update_time)?;

    let mut removed = Table::new(&["tsd_id", "name", "day"], &[]);
    let mut skipped = Table::new(&["tsd_id", "reason"], &[]);
    for tsd_id in stops {
        let Some((day, _, stop)) = locate_stop(&detail, tsd_id) else {
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
    fn moved_order_places_first_last_and_after() {
        let ids: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert_eq!(moved_order(&ids, "c", &MoveTarget::First), ["c", "a", "b"]);
        assert_eq!(moved_order(&ids, "a", &MoveTarget::Last), ["b", "c", "a"]);
        assert_eq!(
            moved_order(&ids, "c", &MoveTarget::After("a".into())),
            ["a", "c", "b"]
        );
        assert_eq!(moved_order(&ids, "b", &MoveTarget::After("a".into())), ids);
        let empty: Vec<String> = Vec::new();
        assert_eq!(moved_order(&empty, "x", &MoveTarget::Last), ["x"]);
    }

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
