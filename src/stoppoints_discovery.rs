use actix_web::{Json, Query, Result, State};
use crate::context::Context;
use gtfs_structures;
use std::borrow::Borrow;

#[derive(Debug, Serialize, Deserialize)]
struct ErrorCondition {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Line {
    line_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Location {
    longitude: f64,
    latitude: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AnnotatedStopPoint {
    stop_point_ref: String,
    stop_name: String,
    lines: Vec<Line>,
    location: Location,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StopPointsDelivery {
    version: String,
    response_time_stamp: String,
    status: bool,
    error_condition: Option<ErrorCondition>,
    annotated_stop_point: Vec<AnnotatedStopPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Siri {
    stop_points_delivery: StopPointsDelivery,
}

impl<'a> From<&'a gtfs_structures::Stop> for AnnotatedStopPoint {
    fn from(stop: &gtfs_structures::Stop) -> Self {
        Self {
            stop_point_ref: stop.id.clone(),
            stop_name: stop.name.clone(),
            lines: vec![],
            location: Location {
                longitude: stop.longitude,
                latitude: stop.latitude,
            },
        }
    }
}

#[derive(Deserialize)]
pub struct Params {
    q: Option<String>,
    #[serde(rename = "BoundingBoxStructure.UpperLeft.Longitude")]
    upper_left_longitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.UpperLeft.Latitude")]
    upper_left_latitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.LowerRight.Latitude")]
    lower_right_longitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.LowerRight.Latitude")]
    lower_right_latitude: Option<f64>,
}

fn name_matches(stop: &gtfs_structures::Stop, q: &str) -> bool {
    stop.name.to_lowercase().contains(q)
}

fn bounding_box_matches(
    stop: &gtfs_structures::Stop,
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
) -> bool {
    stop.longitude >= min_lon
        && stop.longitude <= max_lon
        && stop.latitude >= min_lat
        && stop.latitude <= max_lat
}

pub fn stoppoints_discovery((state, query): (State<Context>, Query<Params>)) -> Result<Json<Siri>> {
    let stops = &state.gtfs.stops;

    let request = query.into_inner();
    let q = request.q.unwrap_or_default().to_lowercase();
    let min_lon = request.upper_left_longitude.unwrap_or(-180.);
    let max_lon = request.lower_right_longitude.unwrap_or(180.);
    let min_lat = request.lower_right_latitude.unwrap_or(-90.);
    let max_lat = request.upper_left_latitude.unwrap_or(90.);

    let filtered = stops
        .values()
        .filter(|s| name_matches(s, &q))
        .filter(|s| bounding_box_matches(s, min_lon, max_lon, min_lat, max_lat))
        .map(|stop| AnnotatedStopPoint::from(stop.borrow()))
        .collect();

    Ok(Json(Siri {
        stop_points_delivery: StopPointsDelivery {
            version: "2.0".to_string(),
            response_time_stamp: chrono::Utc::now().to_rfc3339(),
            annotated_stop_point: filtered,
            error_condition: None,
            status: true,
        },
    }))
}
