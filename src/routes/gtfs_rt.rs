use crate::extractors::RealTimeDatasetWrapper;
use crate::transit_realtime;
use actix_web::{error, http::ContentEncoding, web, HttpResponse};

pub async fn gtfs_rt_protobuf(
    rt_dataset_wrapper: RealTimeDatasetWrapper,
) -> actix_web::Result<web::HttpResponse> {
    use actix_web::dev::BodyEncoding;
    rt_dataset_wrapper
        .gtfs_rt
        .as_ref()
        .ok_or_else(|| error::ErrorNotFound("no realtime data available"))
        .map(|rt| {
            HttpResponse::Ok()
                .content_type("application/x-protobuf")
                .encoding(ContentEncoding::Identity)
                .body(rt.data.clone())
        })
}

pub async fn gtfs_rt_json(
    rt_dataset_wrapper: RealTimeDatasetWrapper,
) -> actix_web::Result<web::Json<transit_realtime::FeedMessage>> {
    use prost::Message;
    rt_dataset_wrapper
        .gtfs_rt
        .as_ref()
        .ok_or_else(|| error::ErrorNotFound("no realtime data available"))
        .map(|rt| rt.data.clone())
        .and_then(|d| {
            transit_realtime::FeedMessage::decode(d.as_slice())
                .map(web::Json)
                .map_err(|e| {
                    error::ErrorInternalServerError(format!(
                        "impossible to decode protobuf message: {}",
                        e
                    ))
                })
        })
}
