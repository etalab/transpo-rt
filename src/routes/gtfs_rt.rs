use crate::actors::{DatasetActor, GetRealtimeDataset};
use crate::transit_realtime;
use actix::Addr;
use actix_web::http::ContentEncoding;
use actix_web::{error, AsyncResponder, Error, HttpRequest, HttpResponse, Json};
use futures::future::Future;

pub fn gtfs_rt(
    req: &HttpRequest<Addr<DatasetActor>>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.state()
        .send(GetRealtimeDataset)
        .map_err(Error::from)
        .and_then(|rt_data| {
            log::info!("Unwrapping rt_data");
            let rt_data = rt_data.unwrap();
            log::info!("rt_data unwrapped");
            match &rt_data.gtfs_rt {
                Some(gtfs_rt) => Ok(HttpResponse::Ok()
                    .content_type("application/x-protobuf")
                    .content_encoding(ContentEncoding::Identity)
                    .body(gtfs_rt.data.clone())),
                None => Ok(HttpResponse::NotFound().body("no realtime data available")),
            }
        })
        .responder()
}

pub fn gtfs_rt_json(
    req: &HttpRequest<Addr<DatasetActor>>,
) -> Box<Future<Item = Json<transit_realtime::FeedMessage>, Error = Error>> {
    use bytes::IntoBuf;
    use prost::Message;
    req.state()
        .send(GetRealtimeDataset)
        .map_err(Error::from)
        .and_then(|rt_data| {
            let rt_data = rt_data.unwrap();

            rt_data
                .gtfs_rt
                .as_ref()
                .ok_or_else(|| error::ErrorNotFound("no realtime data available"))
                .map(|rt| rt.data.clone())
                .and_then(|d| {
                    transit_realtime::FeedMessage::decode(d.into_buf())
                        .map(Json)
                        .map_err(|e| {
                            error::ErrorInternalServerError(format!(
                                "impossible to decode protobuf message: {}",
                                e
                            ))
                        })
                })
        })
        .responder()
}
