use crate::actors::{DatasetActor, GetRealtimeDataset};
use crate::transit_realtime;
use actix::Addr;
use actix_web::{error, http::ContentEncoding, web, HttpResponse};

pub async fn gtfs_rt_protobuf(
    dataset_actor: web::Data<Addr<DatasetActor>>,
) -> actix_web::Result<web::HttpResponse> {
    use actix_web::dev::BodyEncoding;
    let rt_data = dataset_actor.send(GetRealtimeDataset).await.map_err(|e| {
        log::error!("error while querying actor for realtime data: {:?}", e);
        error::ErrorInternalServerError("impossible to get realtime data".to_string())
    })?;
    rt_data
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
    dataset_actor: web::Data<Addr<DatasetActor>>,
) -> actix_web::Result<web::Json<transit_realtime::FeedMessage>> {
    use prost::Message;

    let rt_data = dataset_actor.send(GetRealtimeDataset).await.map_err(|e| {
        log::error!("error while querying actor for realtime data: {:?}", e);
        error::ErrorInternalServerError("impossible to get realtime data".to_string())
    })?;
    rt_data
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
