use crate::context::Context;
use crate::siri_model::{AnnotatedStopPoint, Siri, SiriResponse, StopPointsDelivery};
use actix_web::{Json, Query, Result, State};

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

fn bounding_box_matches(
    coord: &navitia_model::objects::Coord,
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
) -> bool {
    coord.lon >= min_lon && coord.lon <= max_lon && coord.lat >= min_lat && coord.lat <= max_lat
}

pub fn stoppoints_discovery(
    (state, query): (State<Context>, Query<Params>),
) -> Result<Json<SiriResponse>> {
    let arc_data = state.data.clone();
    let data = arc_data.lock().unwrap();
    let model = &data.ntm;

    let request = query.into_inner();
    let q = request.q.unwrap_or_default().to_lowercase();
    let min_lon = request.upper_left_longitude.unwrap_or(-180.);
    let max_lon = request.lower_right_longitude.unwrap_or(180.);
    let min_lat = request.lower_right_latitude.unwrap_or(-90.);
    let max_lat = request.upper_left_latitude.unwrap_or(90.);

    let filtered = model
        .stop_points
        .iter()
        .filter(|(_, s)| s.name.to_lowercase().contains(&q))
        .filter(|(_, s)| bounding_box_matches(&s.coord, min_lon, max_lon, min_lat, max_lat))
        .map(|(id, _)| AnnotatedStopPoint::from(id, &model))
        .collect();

    Ok(Json(SiriResponse {
        siri: Siri {
            stop_points_delivery: Some(StopPointsDelivery {
                version: "2.0".to_string(),
                response_time_stamp: chrono::Utc::now().to_rfc3339(),
                annotated_stop_point: filtered,
                error_condition: None,
                status: true,
            }),
            ..Default::default()
        },
    }))
}
