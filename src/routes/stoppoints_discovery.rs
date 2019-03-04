use crate::actors::{DatasetActor, GetDataset};
use crate::siri_lite::shared::CommonDelivery;
use crate::siri_lite::stop_points_delivery::{AnnotatedStopPoint, StopPointsDelivery};
use crate::siri_lite::{Siri, SiriResponse};
use actix::Addr;
use actix_web::{AsyncResponder, Error, Json, Query, State};
use futures::future::Future;

#[derive(Deserialize, Clone)]
pub struct Params {
    q: Option<String>,
    #[serde(rename = "BoundingBoxStructure.UpperLeft.Longitude")]
    upper_left_longitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.UpperLeft.Latitude")]
    upper_left_latitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.LowerRight.Longitude")]
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

pub fn filter(data: &crate::datasets::Dataset, request: Params) -> SiriResponse {
    let model = &data.ntm;

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

    SiriResponse {
        siri: Siri {
            stop_points_delivery: Some(StopPointsDelivery {
                common: CommonDelivery::default(),
                annotated_stop_point: filtered,
            }),
            ..Default::default()
        },
    }
}

pub fn sp_discovery(
    (actor_addr, query): (State<Addr<DatasetActor>>, Query<Params>),
) -> Box<Future<Item = Json<SiriResponse>, Error = Error>> {
    actor_addr
        .send(GetDataset)
        .map_err(Error::from)
        .and_then(|dataset| dataset.map(|d| Json(filter(&d, query.into_inner()))))
        .responder()
}
