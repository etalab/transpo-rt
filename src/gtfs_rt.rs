use actix_web::http::{ContentEncoding, StatusCode};
use actix_web::{HttpRequest, HttpResponse, Result, error};
use reqwest;
use std::io::Read;

pub fn gtfs_rt(_req: &HttpRequest) -> Result<HttpResponse> {
    let url = std::env::var("URL").expect("cannot find env var URL");

    let pbf = reqwest::get(url.as_str())
        .map_err(|e| error::ErrorInternalServerError(e))?
        .error_for_status()
        .map_err(|e| error::ErrorInternalServerError(e))?;
        
    let buffer: Result<Vec<u8>, _> = pbf.bytes().collect();
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/x-protobuf")
        .content_encoding(ContentEncoding::Identity)
        .body(buffer?))
}
