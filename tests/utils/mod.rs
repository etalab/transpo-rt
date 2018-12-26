use chrono::NaiveDate;
use prost::Message;
use std::sync::{Once, ONCE_INIT};

static LOGGER_INIT: Once = ONCE_INIT;
const SERVER_PATH: &str = "/gtfs_rt";

pub fn make_test_server() -> actix_web::test::TestServer {
    LOGGER_INIT.call_once(|| env_logger::init());
    let begin = NaiveDate::from_ymd(2018, 12, 15);
    let period = transpo_rt::context::Period {
        begin: begin.clone(),
        end: begin.succ(),
    };
    let gtfs_rt_server = mockito::SERVER_URL.to_string() + SERVER_PATH;
    let ctx = transpo_rt::server::make_context("fixtures/gtfs.zip", &gtfs_rt_server, &period);
    let make_server = move || transpo_rt::server::create_server(ctx.clone());

    actix_web::test::TestServer::with_factory(make_server)
}

// Note: as each integration test is build as a separate binary,
// this helper might be seen as dead code for some tests, thus we remove the warning
#[allow(dead_code)]
pub fn run_simple_gtfs_rt_server(
    gtfs_rt: transpo_rt::transit_realtime::FeedMessage,
) -> mockito::Mock {
    let mut buf = vec![];
    gtfs_rt
        .encode(&mut buf)
        .expect("impossible to convert the gtfs_rt to protobuf");
    mockito::mock("GET", SERVER_PATH)
        .with_status(200)
        .with_header("content-type", "application/octet-stream")
        .with_body(buf)
        .create()
}
