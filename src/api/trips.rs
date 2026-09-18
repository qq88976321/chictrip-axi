//! Multi-call recipes over the travel-schedule endpoints. Each one owns the
//! `updateTime` chain the API's optimistic locking requires.

use super::Client;
use super::types::{AddWhereResult, Poi, SystemCover, TripDetail, TripSummary, UserLabel};
use crate::error::{AxiError, ErrorCode};
use serde_json::Value;

/// The zh-TW system label the web app treats as "no label".
const UNLABELED: &str = "\u{672a}\u{6a19}\u{7c64}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Last,
    Best,
}

pub fn list_trips(client: &Client) -> Result<Vec<TripSummary>, AxiError> {
    let data = client.get(
        "TravelSchedule/GetMyAndCollaboration",
        &[
            ("updateTime", "0".to_string()),
            ("orderByColumn", "updatetime".to_string()),
            ("sort", "desc".to_string()),
        ],
    )?;
    decode(data)
}

/// `VerifyUpdateTime` answers 001 or 004; either way `data.updateTime` is the
/// value a mutation has to send.
pub fn current_update_time(client: &Client, trip_id: &str) -> Result<i64, AxiError> {
    let envelope = client.get_envelope(
        "TravelScheduleDetail/VerifyUpdateTime",
        &[
            ("TravelScheduleId", trip_id.to_string()),
            ("travelScheduleUpdateTime", "0".to_string()),
        ],
    )?;
    if !envelope.is_ok() && envelope.status != "004" {
        return Err(envelope.into_error());
    }
    Ok(envelope
        .data
        .get("updateTime")
        .and_then(Value::as_i64)
        .unwrap_or(0))
}

pub fn trip_detail(
    client: &Client,
    trip_id: &str,
    update_time: i64,
) -> Result<TripDetail, AxiError> {
    let data = client.get(
        "TravelScheduleDetail/Get",
        &[
            ("travelScheduleId", trip_id.to_string()),
            ("TravelScheduleUpdateTime", update_time.to_string()),
            ("isMyTravelSchedule", "1".to_string()),
        ],
    )?;
    decode(data)
}

/// Trip preview: chicTrip answers for any trip id with the guest token,
/// shared or not. The parameter is `TravelScheduleId` (capital T); `Get`
/// spells it `travelScheduleId`.
pub fn trip_preview(client: &Client, trip_id: &str) -> Result<TripDetail, AxiError> {
    let data = client.get(
        "TravelScheduleDetail/Preview",
        &[("TravelScheduleId", trip_id.to_string())],
    )?;
    if data.is_null() {
        return Err(AxiError::not_found(format!("no trip with id {trip_id}")));
    }
    decode(data)
}

pub fn default_label_id(client: &Client) -> Result<String, AxiError> {
    let data = client.get("TravelScheduleUserLabel/Get", &[])?;
    let labels: Vec<UserLabel> = decode(data)?;
    let chosen = labels
        .iter()
        .find(|l| l.is_system && l.name.as_deref() == Some(UNLABELED))
        .or_else(|| labels.iter().find(|l| l.is_system))
        .or_else(|| labels.first());
    chosen
        .and_then(|l| l.id.clone())
        .ok_or_else(|| AxiError::internal("chicTrip returned no trip label to file the trip under"))
}

/// AddV2 answers its generic "non-empty request body" error when CoverMediaId
/// is empty; the web app files every new trip under the first system cover.
pub fn default_cover_id(client: &Client) -> Result<String, AxiError> {
    let data = client.get("TravelSchedule/GetSystemCoverList", &[])?;
    let covers: Vec<SystemCover> = decode(data)?;
    covers.first().and_then(|c| c.id.clone()).ok_or_else(|| {
        AxiError::internal("chicTrip returned no system cover to file the trip under")
    })
}

pub fn poi_detail(client: &Client, poi_id: &str) -> Result<Poi, AxiError> {
    let data = client.get("Poi/GetPoiById", &[("id", poi_id.to_string())])?;
    if data.is_null() {
        return Err(AxiError::not_found(format!("no POI with id {poi_id}")));
    }
    decode(data)
}

fn add_where_slot(
    client: &Client,
    trip_id: &str,
    poi_id: &str,
    day: i64,
    position: Position,
) -> Result<String, AxiError> {
    let data = client.get(
        "TravelScheduleDetail/GetAddWhere",
        &[
            ("poiId", poi_id.to_string()),
            ("travelScheduleId", trip_id.to_string()),
            ("travelScheduleUpdateTime", "0".to_string()),
        ],
    )?;
    let result: AddWhereResult = decode(data)?;
    let slots = result
        .day_list
        .iter()
        .find(|d| d.day == Some(day))
        .map(|d| d.add_where_list.clone())
        .unwrap_or_default();
    let chosen = match position {
        Position::Last => slots.last(),
        Position::Best => slots
            .iter()
            .find(|s| s.is_best_of_day)
            .or_else(|| slots.iter().find(|s| s.is_best_of_all))
            .or_else(|| slots.first()),
    };
    chosen
        .and_then(|s| s.add_where_id.clone())
        .ok_or_else(|| AxiError::not_found(format!("day {day} has no slot to add a stop into")))
}

/// Runs `send` with the trip's update time, and once more with a re-read one
/// when chicTrip answers 004: another writer moved the trip on in between.
fn with_update_time<T>(
    client: &Client,
    trip_id: &str,
    update_time: i64,
    mut send: impl FnMut(i64) -> Result<T, AxiError>,
) -> Result<T, AxiError> {
    match send(update_time) {
        Err(e) if e.code == ErrorCode::Conflict => {
            let fresh = current_update_time(client, trip_id)?;
            send(fresh)
        }
        result => result,
    }
}

/// A mutation answers the trip's new update time as a bare integer, or under
/// `travelScheduleUpdateTime` or `updateTime` when it answers an object.
fn new_update_time(data: &Value, fallback: i64) -> i64 {
    data.as_i64()
        .or_else(|| data.get("travelScheduleUpdateTime").and_then(Value::as_i64))
        .or_else(|| data.get("updateTime").and_then(Value::as_i64))
        .unwrap_or(fallback)
}

/// Adds one POI and returns the trip's new update time.
pub fn add_stop(
    client: &Client,
    trip_id: &str,
    day: i64,
    poi: &Poi,
    position: Position,
    update_time: i64,
) -> Result<i64, AxiError> {
    let poi_id = poi.id.clone().unwrap_or_default();
    let slot = add_where_slot(client, trip_id, &poi_id, day, position)?;
    let cover = poi
        .cover
        .as_ref()
        .and_then(|c| c.id.clone())
        .unwrap_or_default();
    let name = poi.name.clone().unwrap_or_default();
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("TravelScheduleId", trip_id.to_string()),
            ("Day", day.to_string()),
            ("PoiId", poi_id.clone()),
            ("AddWhereId", slot.clone()),
            ("TravelScheduleUpdateTime", time.to_string()),
            ("TsdCoverMediaId", cover.clone()),
            ("TsdName", name.clone()),
        ];
        let data = client.post_form("TravelScheduleDetail/Add", &form)?;
        Ok(new_update_time(&data, time))
    })
}

pub fn remove_stop(
    client: &Client,
    trip_id: &str,
    day: i64,
    tsd_id: &str,
    update_time: i64,
) -> Result<i64, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("TravelScheduleId", trip_id.to_string()),
            ("Day", day.to_string()),
            ("TsdId", tsd_id.to_string()),
            ("TravelScheduleUpdateTime", time.to_string()),
        ];
        let data = client.delete_form("TravelScheduleDetail/Delete", &form)?;
        Ok(new_update_time(&data, time))
    })
}

pub fn delete_trip(client: &Client, trip_id: &str) -> Result<(), AxiError> {
    client
        .delete_form("TravelSchedule/Delete", &[("id", trip_id.to_string())])
        .map(|_| ())
}

pub fn copy_tour(client: &Client, tour_id: &str) -> Result<(), AxiError> {
    client
        .post_form_zhtw(
            "ExpertTour/TravelScheduleCopy",
            &[("travelScheduleId", tour_id.to_string())],
        )
        .map(|_| ())
}

fn decode<T: serde::de::DeserializeOwned>(data: Value) -> Result<T, AxiError> {
    serde_json::from_value(data)
        .map_err(|e| AxiError::internal(format!("chicTrip sent an unexpected shape: {e}")))
}
