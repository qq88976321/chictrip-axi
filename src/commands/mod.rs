//! One module per noun. A command fetches, shapes a [`Document`], and returns
//! it; rendering, `--fields`, and exit codes belong to the shared layers.
//!
//! What sits here is what more than one noun needs: flag validation, the
//! optional-to-TOON value conversions, and the tables two commands share.

pub mod auth;
pub mod home;
pub mod location;
pub mod poi;
pub mod setup;
pub mod tour;
pub mod trip;

use crate::api::types::{Day, TripSummary};
use crate::datetime::format_utc_date;
use crate::error::AxiError;
use crate::output::Table;
use serde_json::Value;

pub const MAX_LIMIT: usize = 200;

pub fn validate_limit(limit: usize) -> Result<usize, AxiError> {
    if limit == 0 || limit > MAX_LIMIT {
        return Err(AxiError::usage(format!(
            "--limit must be between 1 and {MAX_LIMIT}"
        )));
    }
    Ok(limit)
}

pub fn parse_near(near: &str) -> Result<(f64, f64), AxiError> {
    let bad = || AxiError::usage(format!("invalid --near '{near}'; expected LAT,LNG"));
    let (lat, lng) = near.split_once(',').ok_or_else(bad)?;
    let lat: f64 = lat.trim().parse().map_err(|_| bad())?;
    let lng: f64 = lng.trim().parse().map_err(|_| bad())?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
        return Err(bad());
    }
    Ok((lat, lng))
}

/// Empty strings are as absent as nulls, and the contract omits both.
pub fn value(text: &Option<String>) -> Value {
    match text {
        Some(text) if !text.is_empty() => Value::from(text.as_str()),
        _ => Value::Null,
    }
}

pub fn number(n: Option<f64>) -> Value {
    n.map(Value::from).unwrap_or(Value::Null)
}

pub fn count(n: Option<i64>) -> Value {
    n.map(Value::from).unwrap_or(Value::Null)
}

/// `count: 20 of 35` once `--limit` trimmed rows, a bare count otherwise.
pub fn count_line(shown: usize, total: usize) -> Value {
    if shown < total {
        Value::from(format!("{shown} of {total}"))
    } else {
        Value::from(shown as i64)
    }
}

pub fn trips_table(trips: &[TripSummary]) -> Table {
    let mut table = Table::new(
        &[
            "id",
            "name",
            "start",
            "end",
            "days",
            "permission",
            "updated",
        ],
        &["traffic", "update_time", "collaborators", "owner"],
    );
    for trip in trips {
        table.push(&[
            ("id", value(&trip.id)),
            ("name", value(&trip.name)),
            ("start", value(&trip.start_date)),
            ("end", value(&trip.end_date)),
            ("days", count(trip.total_day)),
            ("permission", value(&trip.permission)),
            (
                "updated",
                trip.update_time
                    .map(|t| Value::from(format_utc_date(t)))
                    .unwrap_or(Value::Null),
            ),
            ("traffic", value(&trip.traffic_type)),
            ("update_time", count(trip.update_time)),
            (
                "collaborators",
                Value::from(trip.collaboration_list.len() as i64),
            ),
            ("owner", value(&trip.member_id)),
        ]);
    }
    table
}

/// Keeps the requested day, or every day. An out-of-range `--day` is a
/// not_found rather than silently empty output.
pub fn select_days(
    day_list: &[Day],
    day: Option<i64>,
    total_day: Option<i64>,
) -> Result<Vec<Day>, AxiError> {
    let Some(day) = day else {
        return Ok(day_list.to_vec());
    };
    let total = total_day.unwrap_or(day_list.len() as i64);
    if day < 1 || day > total {
        return Err(AxiError::not_found(format!(
            "day {day} is outside this itinerary, which has {total} days"
        )));
    }
    Ok(day_list
        .iter()
        .filter(|d| d.day == Some(day))
        .cloned()
        .collect())
}

/// The stop table shared by `tour view` and `trip view`; `trip view` shows the
/// `tsd_id` that `trip remove` takes.
pub fn stops_table(days: &[Day], full: bool, with_tsd_id: bool) -> Table {
    let mut defaults: Vec<&str> = vec!["day", "seq", "arrive", "stay_min", "name", "type"];
    let mut extras: Vec<&str> = Vec::new();
    if with_tsd_id {
        defaults.push("tsd_id");
    } else {
        extras.push("tsd_id");
    }
    defaults.push("city");
    defaults.push("poi_id");
    let detail_columns = ["note", "traffic", "traffic_min", "flight"];
    if full {
        defaults.extend(detail_columns);
    } else {
        extras.extend(detail_columns);
    }
    let mut table = Table::new(&defaults, &extras);

    for day in days {
        for (index, stop) in day.tsd_list.iter().enumerate() {
            table.push(&[
                ("day", count(day.day.or(stop.day))),
                ("seq", Value::from(index as i64 + 1)),
                ("arrive", value(&stop.arrival_time)),
                ("stay_min", count(stop.stay_time)),
                ("name", value(&stop.name)),
                ("type", value(&stop.tsd_type)),
                ("tsd_id", value(&stop.id)),
                ("city", value(&stop.city_name)),
                ("poi_id", value(&stop.poi_id)),
                ("note", value(&stop.note)),
                ("traffic", value(&stop.arrival_traffic_type)),
                ("traffic_min", count(stop.arrival_traffic_time)),
                ("flight", value(&stop.flight_number)),
            ]);
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::Tsd;
    use crate::output::{Document, render};

    fn days() -> Vec<Day> {
        vec![
            Day {
                day: Some(1),
                date: Some("2026/10/01".into()),
                tsd_list: vec![Tsd {
                    id: Some("t1".into()),
                    name: Some("Kaminarimon".into()),
                    tsd_type: Some("poi".into()),
                    ..Tsd::default()
                }],
                ..Day::default()
            },
            Day {
                day: Some(2),
                date: Some("2026/10/02".into()),
                tsd_list: vec![],
                ..Day::default()
            },
        ]
    }

    #[test]
    fn limits_are_bounded() {
        assert_eq!(validate_limit(1).unwrap(), 1);
        assert_eq!(validate_limit(MAX_LIMIT).unwrap(), MAX_LIMIT);
        assert!(validate_limit(0).is_err());
        assert!(validate_limit(MAX_LIMIT + 1).is_err());
    }

    #[test]
    fn near_wants_two_numbers_in_range() {
        assert_eq!(parse_near("35.71,139.79").unwrap(), (35.71, 139.79));
        assert_eq!(parse_near(" 0 , 0 ").unwrap(), (0.0, 0.0));
        assert!(parse_near("35.71").is_err());
        assert!(parse_near("here,there").is_err());
        assert!(parse_near("100,0").is_err());
    }

    #[test]
    fn counts_say_how_many_rows_were_trimmed() {
        assert_eq!(count_line(12, 12), Value::from(12));
        assert_eq!(count_line(20, 35), Value::from("20 of 35"));
    }

    #[test]
    fn day_filtering_keeps_one_day_and_rejects_the_rest() {
        assert_eq!(select_days(&days(), None, Some(2)).unwrap().len(), 2);
        assert_eq!(select_days(&days(), Some(2), Some(2)).unwrap().len(), 1);
        assert!(select_days(&days(), Some(3), Some(2)).is_err());
        assert!(select_days(&days(), Some(0), Some(2)).is_err());
    }

    #[test]
    fn trip_stops_carry_the_tsd_id_and_tour_stops_do_not() {
        let mut doc = Document::new();
        doc.set_table("stops", stops_table(&days(), false, true));
        assert!(render(&doc, false).contains("tsd_id"));
        let mut doc = Document::new();
        doc.set_table("stops", stops_table(&days(), false, false));
        assert!(!render(&doc, false).contains("tsd_id"));
    }

    #[test]
    fn the_trip_table_renders_the_update_time_as_a_utc_day() {
        let trips = vec![TripSummary {
            id: Some("t1".into()),
            name: Some("Tokyo".into()),
            start_date: Some("2026/10/01".into()),
            end_date: Some("2026/10/03".into()),
            total_day: Some(3),
            permission: Some("Owner".into()),
            update_time: Some(1_716_359_728),
            ..TripSummary::default()
        }];
        let mut doc = Document::new();
        doc.set_table("trips", trips_table(&trips));
        let rendered = render(&doc, false);
        assert!(rendered.contains("trips[1]{id,name,start,end,days,permission,updated}:"));
        assert!(rendered.contains("2024-05-22"));
    }
}
