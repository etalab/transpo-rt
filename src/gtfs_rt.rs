use actix_web::http::{ContentEncoding, StatusCode};
use actix_web::{HttpRequest, HttpResponse, Result};
use reqwest;
use std::io::Read;

pub fn gtfs_rt(_req: &HttpRequest) -> Result<HttpResponse> {
    let url = std::env::var("URL").expect("cannot find env var URL");

    let mut pbf = reqwest::get(url.as_str())
        .unwrap()
        .error_for_status()
        .unwrap();

    let mut buffer = vec![];
    pbf.read_to_end(&mut buffer).unwrap();

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/x-protobuf")
        .content_encoding(ContentEncoding::Identity)
        .body(buffer))
}
