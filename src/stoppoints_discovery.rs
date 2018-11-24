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
}

pub fn stoppoints_discovery((state, query): (State<Context>, Query<Params>)) -> Result<Json<Siri>> {
    let stops = &state.gtfs.stops;

    let q = query.into_inner().q.unwrap_or_default().to_lowercase();
    let filtered = stops
        .values()
        .filter(|stop| stop.name.to_lowercase().contains(q.as_str()))
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
