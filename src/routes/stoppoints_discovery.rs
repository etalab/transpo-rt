use super::open_api::make_param;
use crate::actors::{DatasetActor, GetDataset};
use crate::siri_lite::shared::CommonDelivery;
use crate::siri_lite::stop_points_delivery::{AnnotatedStopPoint, StopPointsDelivery};
use crate::siri_lite::{Siri, SiriResponse};
use actix::Addr;
use actix_web::{get, web};

fn default_limit() -> usize {
    20
}

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
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

impl Params {
    pub fn openapi_description(spec: &mut openapi::v3_0::Spec) -> Vec<openapi::v3_0::Parameter> {
        vec![
            make_param::<String>(spec, "q", false),
            make_param::<f64>(spec, "BoundingBoxStructure.UpperLeft.Longitude", false),
            make_param::<f64>(spec, "BoundingBoxStructure.UpperLeft.Latitude", false),
            make_param::<f64>(spec, "BoundingBoxStructure.LowerRight.Longitude", false),
            make_param::<f64>(spec, "BoundingBoxStructure.LowerRight.Latitude", false),
            make_param::<usize>(spec, "limit", false),
            make_param::<usize>(spec, "offset", false),
        ]
    }
}

fn bounding_box_matches(
    coord: &transit_model::objects::Coord,
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
        .skip(request.offset)
        .take(request.limit)
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

#[get("/siri/2.0/stoppoints-discovery.json")]
pub async fn stoppoints_discovery_query(
    web::Query(query): web::Query<Params>,
    dataset_actor: web::Data<Addr<DatasetActor>>,
) -> actix_web::Result<web::Json<SiriResponse>> {
    let dataset = dataset_actor.send(GetDataset).await.map_err(|e| {
        log::error!("error while querying actor for data: {:?}", e);
        actix_web::error::ErrorInternalServerError("impossible to get data".to_string())
    })?;
    Ok(web::Json(filter(&dataset, query)))
}
