use crate::context::Context;
use crate::gtfs_rt_utils::get_gtfs_rt;
use crate::transit_realtime;
use actix_web::http::{ContentEncoding, StatusCode};
use actix_web::{error, HttpRequest, HttpResponse, Json, Result};
use bytes::IntoBuf;
use prost::Message;

pub fn gtfs_rt(req: &HttpRequest<Context>) -> Result<HttpResponse> {
    let saved_data = get_gtfs_rt(req.state()).map_err(error::ErrorInternalServerError)?;

    let data: Vec<u8> = saved_data
        .as_ref()
        .map(|d| d.data.clone())
        .ok_or_else(|| error::ErrorInternalServerError("impossible to access stored data"))?;

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/x-protobuf")
        .content_encoding(ContentEncoding::Identity)
        .body(data))
}

pub fn gtfs_rt_json(req: &HttpRequest<Context>) -> Result<Json<transit_realtime::FeedMessage>> {
    let saved_data = get_gtfs_rt(req.state()).map_err(error::ErrorInternalServerError)?;
    let data = saved_data
        .as_ref()
        .map(|d| {
            transit_realtime::FeedMessage::decode((&d.data).into_buf()).map_err(|e| {
                error::ErrorInternalServerError(format!(
                    "impossible to decode protobuf message: {}",
                    e
                ))
            })
        })
        .ok_or_else(|| error::ErrorInternalServerError("impossible to access stored data"))?;

    Ok(Json(data?))
}
