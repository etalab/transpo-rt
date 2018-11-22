use actix_web::http::{ContentEncoding, StatusCode};
use actix_web::{error, HttpRequest, HttpResponse, Result};
use chrono::Utc;
use crate::state::{GtfsRT, State};
use failure::Error;
use reqwest;
use std::io::Read;

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

pub fn gtfs_rt(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let mut saved_data = req.state().gtfs_rt.lock().unwrap();
    if refresh_needed(&saved_data) {
        *saved_data = Some(GtfsRT {
            data: fetch_gtfs().map_err(|e| error::ErrorInternalServerError(e))?,
            datetime: Utc::now(),
        });
    }
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
