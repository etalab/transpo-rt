use actix_web::http::{ContentEncoding, StatusCode};
use actix_web::{error, HttpRequest, HttpResponse, Json, Result};
use bytes::IntoBuf;
use chrono::Utc;
use crate::context::{Context, GtfsRT};
use crate::transit_realtime;
use failure::Error;
use prost::Message;
use reqwest;
use std::io::Read;
use std::sync::MutexGuard;

const REFRESH_TIMEOUT_S: i64 = 60;

fn fetch_gtfs() -> Result<Vec<u8>, Error> {
    info!("fetching a gtfs_rt");
    let url = std::env::var("URL").expect("cannot find env var URL");

    let pbf = reqwest::get(url.as_str())?.error_for_status()?;

    pbf.bytes()
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| e.into())
}

fn refresh_needed(previous: &Option<GtfsRT>) -> bool {
    previous
        .as_ref()
        .map(|g| g.datetime)
        .map(|dt| (chrono::Utc::now() - dt).num_seconds().abs() > REFRESH_TIMEOUT_S)
        .unwrap_or(true)
}

fn get_gtfs_rt(context: &Context) -> Result<MutexGuard<Option<GtfsRT>>, Error> {
    let mut saved_data = context.gtfs_rt.lock().unwrap();
    if refresh_needed(&saved_data) {
        *saved_data = Some(GtfsRT {
            data: fetch_gtfs()?,
            datetime: Utc::now(),
        });
    }
    Ok(saved_data)
}

pub fn gtfs_rt(req: &HttpRequest<Context>) -> Result<HttpResponse> {
    let saved_data = get_gtfs_rt(req.state()).map_err(|e| error::ErrorInternalServerError(e))?;

    let data: Vec<u8> =
        saved_data
            .as_ref()
            .map(|d| d.data.clone())
            .ok_or(error::ErrorInternalServerError(
                "impossible to access stored data",
            ))?;

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/x-protobuf")
        .content_encoding(ContentEncoding::Identity)
        .body(data))
}

pub fn gtfs_rt_json(req: &HttpRequest<Context>) -> Result<Json<transit_realtime::FeedMessage>> {
    let saved_data = get_gtfs_rt(req.state()).map_err(|e| error::ErrorInternalServerError(e))?;
    let data = saved_data
        .as_ref()
        .map(|d| {
            transit_realtime::FeedMessage::decode((&d.data).into_buf()).map_err(|e| {
                error::ErrorInternalServerError(format!(
                    "impossible to decode protobuf message: {}",
                    e
                ))
            })
        }).ok_or(error::ErrorInternalServerError(
            "impossible to access stored data",
        ))?;

    Ok(Json(data?))
}
