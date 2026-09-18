//! Multi-call recipes over the travel-schedule endpoints. Each one owns the
//! `updateTime` chain the API's optimistic locking requires.

use super::Client;
use super::types::{
    AddWhereResult, AddedStop, DayTraffic, EditInfo, Poi, RouteList, SystemCover, TripDetail,
    TripSummary, UserLabel,
};
use crate::error::{AxiError, ErrorCode};
use serde_json::Value;

/// The zh-TW system label the web app treats as "no label".
const UNLABELED: &str = "\u{672a}\u{6a19}\u{7c64}";

/// What `GetEditInfo` answers for a tsd id the trip does not hold: a 002
/// with this message and a null payload, not the 001 a miss would suggest.
const TSD_NOT_FOUND: &str = "TSD Id not found";

/// Where `GetAddWhere` can put a new stop. A day's slots are `start`, one
/// named after each stop EXCEPT the first, and `end`; an empty day has a
/// single slot named `first`. A slot named after a stop puts the new one in
/// FRONT of it, so `Slot` carries an `addWhereId` verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    Last,
    Best,
    Slot(String),
}

/// How a leg is travelled. `TRAFFIC_MODES` in `commands/trip.rs` is AddV2's
/// own, smaller vocabulary and is not this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficMode {
    Custom,
    Driving,
    Transit,
    Walking,
    TwoWheeler,
    Flight,
}

impl TrafficMode {
    /// The modes chicTrip can compute routes for.
    pub const ROUTABLE: [TrafficMode; 4] = [
        TrafficMode::Driving,
        TrafficMode::Transit,
        TrafficMode::Walking,
        TrafficMode::TwoWheeler,
    ];

    pub fn api(self) -> &'static str {
        match self {
            TrafficMode::Custom => "Custom",
            TrafficMode::Driving => "Driving",
            TrafficMode::Transit => "Transit",
            TrafficMode::Walking => "Walking",
            TrafficMode::TwoWheeler => "TwoWheeler",
            TrafficMode::Flight => "Flight",
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            TrafficMode::TwoWheeler => "scooter",
            TrafficMode::Custom => "custom",
            TrafficMode::Driving => "driving",
            TrafficMode::Transit => "transit",
            TrafficMode::Walking => "walking",
            TrafficMode::Flight => "flight",
        }
    }

    pub fn is_routable(self) -> bool {
        Self::ROUTABLE.contains(&self)
    }

    /// Accepts the CLI token or chicTrip's own spelling, case-insensitively.
    pub fn from_token(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| text.eq_ignore_ascii_case(mode.token()) || Self::spells(*mode, text))
    }

    pub fn from_api(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| Self::spells(*mode, text))
    }

    const ALL: [TrafficMode; 6] = [
        TrafficMode::Custom,
        TrafficMode::Driving,
        TrafficMode::Transit,
        TrafficMode::Walking,
        TrafficMode::TwoWheeler,
        TrafficMode::Flight,
    ];

    fn spells(mode: TrafficMode, text: &str) -> bool {
        text.eq_ignore_ascii_case(mode.api())
    }
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
    position: &Position,
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
        Position::Slot(id) => slots
            .iter()
            .find(|s| s.add_where_id.as_deref() == Some(id.as_str())),
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

/// Adds one POI and returns its new tsd id and the trip's new update time.
pub fn add_stop(
    client: &Client,
    trip_id: &str,
    day: i64,
    poi: &Poi,
    position: &Position,
    update_time: i64,
) -> Result<(String, i64), AxiError> {
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
        let new_time = new_update_time(&data, time);
        let added: AddedStop = decode(data)?;
        Ok((added.tsd_info.id.unwrap_or_default(), new_time))
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

/// The whole edit sheet of one stop. `Update` wants every field back, so
/// this is what the command merges its flags into.
pub struct StopEdit {
    pub name: String,
    pub category_id: String,
    pub stay_time: i64,
    /// `None` means chicTrip computes the time.
    pub arrival: Option<String>,
    pub departure: Option<String>,
}

pub fn edit_info(
    client: &Client,
    trip_id: &str,
    tsd_id: &str,
    update_time: i64,
) -> Result<EditInfo, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let envelope = client.get_envelope(
            "TravelScheduleDetail/GetEditInfo",
            &[
                ("travelScheduleId", trip_id.to_string()),
                ("tsdId", tsd_id.to_string()),
                ("travelScheduleUpdateTime", time.to_string()),
            ],
        )?;
        if envelope.message.as_deref() == Some(TSD_NOT_FOUND) {
            return Err(AxiError::not_found(format!(
                "stop {tsd_id} is not in trip {trip_id}"
            )));
        }
        decode(envelope.into_data()?)
    })
}

pub fn update_stop(
    client: &Client,
    trip_id: &str,
    tsd_id: &str,
    edit: &StopEdit,
    update_time: i64,
) -> Result<i64, AxiError> {
    let pinned = |time: &Option<String>| if time.is_some() { "1" } else { "0" }.to_string();
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("TsdId", tsd_id.to_string()),
            ("Name", edit.name.clone()),
            ("PoiClassificationId", edit.category_id.clone()),
            ("StayTime", edit.stay_time.to_string()),
            ("IsUseCustomArrivalTime", pinned(&edit.arrival)),
            (
                "CustomArrivalTime",
                edit.arrival.clone().unwrap_or_default(),
            ),
            ("IsUseCustomDepartureTime", pinned(&edit.departure)),
            (
                "CustomDepartureTime",
                edit.departure.clone().unwrap_or_default(),
            ),
            ("TravelScheduleId", trip_id.to_string()),
            ("travelScheduleUpdateTime", time.to_string()),
        ];
        let data = client.put_form("TravelScheduleDetail/Update", &form)?;
        Ok(new_update_time(&data, time))
    })
}

pub fn set_stop_note(
    client: &Client,
    trip_id: &str,
    tsd_id: &str,
    note: &str,
    update_time: i64,
) -> Result<i64, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("TravelScheduleId", trip_id.to_string()),
            ("TsdId", tsd_id.to_string()),
            ("Note", note.to_string()),
            ("TravelScheduleUpdateTime", time.to_string()),
        ];
        let data = client.put_form("TravelScheduleDetail/UpdateNote", &form)?;
        Ok(new_update_time(&data, time))
    })
}

pub fn set_trip_note(
    client: &Client,
    trip_id: &str,
    note: &str,
    update_time: i64,
) -> Result<i64, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("id", trip_id.to_string()),
            ("note", note.to_string()),
            ("updateTime", time.to_string()),
        ];
        let data = client.put_form("TravelSchedule/UpdateNote", &form)?;
        Ok(new_update_time(&data, time))
    })
}

pub fn route_list(
    client: &Client,
    trip_id: &str,
    route_id: &str,
    mode: TrafficMode,
    update_time: i64,
) -> Result<RouteList, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let data = client.get(
            "TravelScheduleDetailRoute/GetRouteList",
            &[
                ("tsdRouteDetailId", route_id.to_string()),
                ("trafficType", mode.api().to_string()),
                ("travelScheduleId", trip_id.to_string()),
                ("TravelScheduleUpdateTime", time.to_string()),
            ],
        )?;
        decode(data)
    })
}

pub fn set_route(
    client: &Client,
    trip_id: &str,
    route_id: &str,
    poi_route_id: &str,
    update_time: i64,
) -> Result<i64, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("TsdRouteDetailId", route_id.to_string()),
            ("PoiRouteDetailId", poi_route_id.to_string()),
            ("TravelScheduleId", trip_id.to_string()),
            ("travelScheduleUpdateTime", time.to_string()),
        ];
        let data = client.put_form("TravelScheduleDetail/SetRoute", &form)?;
        Ok(new_update_time(&data, time))
    })
}

pub fn set_custom_route(
    client: &Client,
    trip_id: &str,
    route_id: &str,
    minutes: i64,
    note: &str,
    flight: bool,
    update_time: i64,
) -> Result<i64, AxiError> {
    let path = if flight {
        "TravelScheduleDetail/SetFlightRoute"
    } else {
        "TravelScheduleDetail/SetCustomRoute"
    };
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("TsdRouteDetailId", route_id.to_string()),
            ("Duration", minutes.to_string()),
            ("Note", note.to_string()),
            ("TravelScheduleId", trip_id.to_string()),
            ("travelScheduleUpdateTime", time.to_string()),
        ];
        let data = client.put_form(path, &form)?;
        Ok(new_update_time(&data, time))
    })
}

/// Sets a whole day's default mode. `recompute` also re-routes the legs that
/// were picked or set by hand; without it only new legs pick the mode up.
pub fn set_day_traffic(
    client: &Client,
    trip_id: &str,
    day: i64,
    mode: TrafficMode,
    recompute: bool,
    update_time: i64,
) -> Result<DayTraffic, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let form = [
            ("travelScheduleId", trip_id.to_string()),
            ("day", day.to_string()),
            ("trafficType", mode.api().to_string()),
            ("travelScheduleUpdateTime", time.to_string()),
            (
                "isForceUpdateTsdRoute",
                if recompute { "1" } else { "0" }.to_string(),
            ),
        ];
        let data = client.put_form("TravelScheduleDetail/SetDefaultRouteAndTsdAllDay", &form)?;
        decode(data)
    })
}

/// Reorders a day. `order` is the WHOLE target day in its new order, the
/// moved stop included; `from_day` and `to_day` differ when the stop crosses
/// days, which chicTrip accepts and which keeps the stop's id and note
/// (verified live 2026-09-19).
pub fn sort_day(
    client: &Client,
    trip_id: &str,
    from_day: i64,
    to_day: i64,
    tsd_id: &str,
    order: &[String],
    update_time: i64,
) -> Result<i64, AxiError> {
    with_update_time(client, trip_id, update_time, |time| {
        let mut form = vec![
            ("TravelScheduleId", trip_id.to_string()),
            ("MoveOutDay", from_day.to_string()),
            ("MoveInDay", to_day.to_string()),
            ("MoveTsdId", tsd_id.to_string()),
        ];
        for id in order {
            form.push(("TsdIdList[]", id.clone()));
        }
        form.push(("travelScheduleUpdateTime", time.to_string()));
        let data = client.put_form("TravelScheduleDetail/Sort", &form)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traffic_modes_round_trip_between_tokens_and_api_values() {
        assert_eq!(
            TrafficMode::from_token("scooter"),
            Some(TrafficMode::TwoWheeler)
        );
        assert_eq!(
            TrafficMode::from_token("TwoWheeler"),
            Some(TrafficMode::TwoWheeler)
        );
        assert_eq!(
            TrafficMode::from_token("TRANSIT"),
            Some(TrafficMode::Transit)
        );
        assert_eq!(TrafficMode::from_token("bus"), None);
        assert_eq!(TrafficMode::from_api("Flight"), Some(TrafficMode::Flight));
        assert_eq!(TrafficMode::from_api("scooter"), None);
        assert_eq!(TrafficMode::TwoWheeler.api(), "TwoWheeler");
        assert!(!TrafficMode::Custom.is_routable());
        assert!(!TrafficMode::Flight.is_routable());
        assert!(TrafficMode::ROUTABLE.iter().all(|m| m.is_routable()));
    }
}
