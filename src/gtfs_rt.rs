use crate::context::Dataset;
use crate::dataset_handler_actor::{DatasetActor, GetDataset};
use crate::gtfs_rt_utils::get_gtfs_rt;
use crate::transit_realtime;
use actix::Addr;
use actix_web::http::ContentEncoding;
use actix_web::{error, AsyncResponder, Error, HttpRequest, HttpResponse, Json, Result};
use bytes::IntoBuf;
use futures::future::Future;
use prost::Message;

pub fn gtfs_rt(
    req: &HttpRequest<Addr<DatasetActor>>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.state()
        .send(GetDataset)
        .map_err(Error::from)
        .and_then(|dataset| {
            dataset.and_then(|d| get_gtfs_rt_bin(&*d)).map(|d| {
                HttpResponse::Ok()
                    .content_type("application/x-protobuf")
                    .content_encoding(ContentEncoding::Identity)
                    .body(d)
            })
        })
        .responder()
}

pub fn gtfs_rt_json(
    req: &HttpRequest<Addr<DatasetActor>>,
) -> Box<Future<Item = Json<transit_realtime::FeedMessage>, Error = Error>> {
    req.state()
        .send(GetDataset)
        .map_err(Error::from)
        .and_then(|dataset| {
            dataset.and_then(|d| get_gtfs_rt_bin(&*d)).and_then(|d| {
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

fn get_gtfs_rt_bin(dataset: &Dataset) -> Result<Vec<u8>> {
    let saved_data = get_gtfs_rt(dataset).map_err(error::ErrorInternalServerError)?;

    saved_data
        .as_ref()
        .map(|d| d.data.clone())
        .ok_or_else(|| error::ErrorInternalServerError("impossible to access stored data"))
}
