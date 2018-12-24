use crate::context::Context;
use crate::gtfs_rt_utils::get_gtfs_rt;
use crate::transit_realtime;
use actix::{Addr, Handler};
use actix_web::http::ContentEncoding;
use actix_web::{error, AsyncResponder, Error, HttpRequest, HttpResponse, Json, Result};
use bytes::IntoBuf;
use futures::future::Future;
use prost::Message;

struct Params;

pub fn gtfs_rt(
    req: &HttpRequest<Addr<Context>>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.state()
        .send(Params {})
        .map_err(Error::from)
        .and_then(|result| {
            result.map(|data| {
                HttpResponse::Ok()
                    .content_type("application/x-protobuf")
                    .content_encoding(ContentEncoding::Identity)
                    .body(data)
            })
        })
        .responder()
}

pub fn gtfs_rt_json(
    req: &HttpRequest<Addr<Context>>,
) -> Box<Future<Item = Json<transit_realtime::FeedMessage>, Error = Error>> {
    req.state()
        .send(Params {})
        .map_err(Error::from)
        .and_then(|result| {
            result.and_then(|data| {
                transit_realtime::FeedMessage::decode(data.into_buf())
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

impl actix::Message for Params {
    type Result = Result<Vec<u8>>;
}
impl Handler<Params> for Context {
    type Result = Result<Vec<u8>>;

    fn handle(&mut self, _params: Params, _ctx: &mut actix::Context<Self>) -> Self::Result {
        let saved_data = get_gtfs_rt(self).map_err(error::ErrorInternalServerError)?;

        saved_data
            .as_ref()
            .map(|d| d.data.clone())
            .ok_or_else(|| error::ErrorInternalServerError("impossible to access stored data"))
    }
}
