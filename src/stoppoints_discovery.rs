use crate::context::Context;
use crate::siri_model::{AnnotatedStopPoint, Siri, SiriResponse, StopPointsDelivery};
use actix::{Addr, Handler, Message};
use actix_web::{AsyncResponder, Error, Json, Query, Result, State};
use futures::future::Future;

#[derive(Deserialize, Clone)]
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

pub fn filter(data: &crate::context::Data, request: Params) -> SiriResponse {
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
                version: "2.0".to_string(),
                response_time_stamp: chrono::Utc::now().to_rfc3339(),
                annotated_stop_point: filtered,
                error_condition: None,
                status: true,
            }),
            ..Default::default()
        },
    }
}

pub fn sp_discovery(
    (actor_addr, query): (State<Addr<Context>>, Query<Params>),
) -> Box<Future<Item = Json<SiriResponse>, Error = Error>> {
    actor_addr
        .send(query.into_inner())
        .map_err(Error::from)
        .and_then(|result| result.map(Json))
        .responder()
}

impl Message for Params {
    type Result = Result<SiriResponse>;
}

impl Handler<Params> for Context {
    type Result = Result<SiriResponse>;

    fn handle(&mut self, params: Params, _ctx: &mut actix::Context<Self>) -> Self::Result {
        let arc_data = self.data.clone();
        let data = arc_data.lock().unwrap();
        Ok(filter(&data, params))
    }
}
