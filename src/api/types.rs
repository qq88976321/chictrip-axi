//! Response shapes for the endpoints the CLI uses. Everything is optional and
//! unknown fields are ignored: chicTrip publishes no API contract and adds
//! fields without notice.

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Location {
    pub location_country_name: Option<String>,
    pub location_city_name: Option<String>,
    pub location_area_name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tag {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Media {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    pub source: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PoiTicket {
    pub title: Option<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OpenTime {
    pub code: Option<String>,
    pub descript: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Poi {
    pub id: Option<String>,
    pub name: Option<String>,
    pub category_type: Option<String>,
    pub address: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    /// The average score, despite the name; `rating_total` is the review count.
    pub rating_count: Option<f64>,
    pub rating_total: Option<i64>,
    pub favorite_count: Option<i64>,
    pub join_count: Option<i64>,
    pub place_id: Option<String>,
    pub location: Location,
    pub tags: Vec<Tag>,
    pub description: Option<String>,
    pub phone_number: Option<String>,
    pub url: Option<String>,
    pub open_times: Vec<OpenTime>,
    pub media: Vec<Media>,
    pub poi_tickets: Vec<PoiTicket>,
    pub cover: Option<Media>,
}

impl Poi {
    pub fn tag_names(&self) -> Vec<String> {
        self.tags.iter().filter_map(|t| t.name.clone()).collect()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PoiSearchResult {
    pub has_next_page: bool,
    pub result: Vec<Poi>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TourListItem {
    pub travel_schedule_id: Option<String>,
    pub name: Option<String>,
    pub destination: Option<String>,
    pub expert_name: Option<String>,
    pub like_count: Option<i64>,
    pub used_count: Option<i64>,
    pub tags: Vec<String>,
    /// Only the curated list carries one.
    pub introduction: Option<String>,
    /// The curated list spells its tags as one comma-joined string.
    pub tag: Option<String>,
}

impl TourListItem {
    pub fn tag_list(&self) -> Vec<String> {
        if !self.tags.is_empty() {
            return self.tags.clone();
        }
        self.tag
            .iter()
            .flat_map(|t| t.split(','))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PopularRanking {
    pub next_page: Option<i64>,
    pub expert_tour_detail_list: Vec<TourListItem>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HighlightPoi {
    pub poi_id: Option<String>,
    pub poi_name: Option<String>,
    pub rating_count: Option<f64>,
    pub join_count: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TourOverview {
    pub travel_schedule_id: Option<String>,
    pub name: Option<String>,
    pub destination: Option<String>,
    pub total_day: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub introduction: Option<String>,
    pub expert_name: Option<String>,
    pub like_count: Option<i64>,
    pub used_count: Option<i64>,
    pub you_tube_url: Option<String>,
    pub highlight_poi_list: Vec<HighlightPoi>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tsd {
    pub id: Option<String>,
    pub name: Option<String>,
    pub tsd_type: Option<String>,
    pub poi_id: Option<String>,
    pub day: Option<i64>,
    pub city_name: Option<String>,
    pub category_icon: Option<String>,
    pub note: Option<String>,
    pub arrival_time: Option<String>,
    pub stay_time: Option<i64>,
    pub arrival_traffic_type: Option<String>,
    pub arrival_traffic_time: Option<i64>,
    pub flight_number: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Day {
    pub day: Option<i64>,
    pub date: Option<String>,
    pub tsd_list: Vec<Tsd>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TourDetail {
    pub overview: TourOverview,
    pub day_list: Vec<Day>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TripSummary {
    pub id: Option<String>,
    pub member_id: Option<String>,
    pub name: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub total_day: Option<i64>,
    pub traffic_type: Option<String>,
    pub update_time: Option<i64>,
    pub permission: Option<String>,
    pub collaboration_list: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TripInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub total_day: Option<i64>,
    pub traffic_type: Option<String>,
    pub update_time: Option<i64>,
    pub permission: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TripDetail {
    pub travel_schedule_info: TripInfo,
    pub day_list: Vec<Day>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemCover {
    pub id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserLabel {
    pub id: Option<String>,
    pub name: Option<String>,
    pub is_system: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AddWhere {
    pub add_where_id: Option<String>,
    pub sort: Option<i64>,
    pub is_best_of_day: bool,
    pub is_best_of_all: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AddWhereDay {
    pub day: Option<i64>,
    pub add_where_list: Vec<AddWhere>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AddWhereResult {
    pub day_list: Vec<AddWhereDay>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreatedTrip {
    pub id: Option<String>,
    pub name: Option<String>,
    pub update_time: Option<i64>,
    pub permission: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let poi: Poi = serde_json::from_str(r#"{"id":"a","surprise":{"x":1}}"#).unwrap();
        assert_eq!(poi.id.as_deref(), Some("a"));
        assert!(poi.tags.is_empty());
        assert!(poi.rating_count.is_none());
    }

    #[test]
    fn curated_tours_split_their_joined_tag_string() {
        let tour: TourListItem = serde_json::from_str(r#"{"tag":"picked, japan ,"}"#).unwrap();
        assert_eq!(tour.tag_list(), vec!["picked", "japan"]);
    }
}
