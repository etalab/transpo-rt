use prost::Message;
use transpo_rt::datasets::{DatasetInfo, Datasets};

const SERVER_PATH: &str = "/gtfs_rt";

pub fn init_log() -> slog_scope::GlobalLoggerGuard {
    use slog::Drain;
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let log = slog::Logger::root(slog_term::FullFormat::new(plain).build().fuse(), slog::o!());
    slog_scope::set_global_logger(log)
}

#[allow(dead_code)]
pub async fn make_simple_test_server() -> actix_web::test::TestServer {
    make_test_server(vec![DatasetInfo::new_default(
        "fixtures/gtfs.zip",
        &[mockito::server_url() + SERVER_PATH],
    )])
    .await
}

pub async fn make_test_server(datasets_info: Vec<DatasetInfo>) -> actix_web::test::TestServer {
    let period = transpo_rt::datasets::Period {
        begin: chrono::NaiveDate::from_ymd(2018, 12, 15),
        horizon: chrono::Duration::days(1),
    };

    let dataset_infos = Datasets {
        datasets: datasets_info.clone(),
    };
    let actors = transpo_rt::server::create_all_actors(dataset_infos, &period).await;
    actix_web::test::start(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::normalize::NormalizePath::default())
            .configure(|cfg| transpo_rt::server::init_routes(cfg, &actors))
    })
}

/// utils to query a route and get the response as json
/// It also assert that the response's status is ok
#[allow(dead_code)]
pub async fn get_json<T: serde::de::DeserializeOwned>(
    srv: &mut actix_web::test::TestServer,
    route: &str,
) -> T {
    let mut response = srv.get(route).send().await.unwrap();
    assert!(response.status().is_success());

    response.json().await.unwrap()
}

// Note: as each integration test is build as a separate binary,
// this helper might be seen as dead code for some tests, thus we remove the warning
#[allow(dead_code)]
pub fn run_simple_gtfs_rt_server(
    gtfs_rt: transpo_rt::transit_realtime::FeedMessage,
) -> mockito::Mock {
    run_gtfs_rt_server(SERVER_PATH, gtfs_rt)
}

#[allow(dead_code)]
pub fn run_gtfs_rt_server(
    path: &str,
    gtfs_rt: transpo_rt::transit_realtime::FeedMessage,
) -> mockito::Mock {
    let mut buf = vec![];
    gtfs_rt
        .encode(&mut buf)
        .expect("impossible to convert the gtfs_rt to protobuf");
    mockito::mock("GET", path)
        .with_status(200)
        .with_header("content-type", "application/octet-stream")
        .with_body(buf)
        .create()
}

// take a date (formated as YYYY-MM-DDTHH:MM:SS) and convert it to a timestamp
#[allow(dead_code)]
pub fn to_timestamp(date: &str) -> i64 {
    chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(date)
        .expect("impossible to parse datetime")
        .timestamp()
}

#[allow(dead_code)]
pub fn make_stu(
    stop_name: &str,
    stop_sequence: u32,
    arrival: Option<&str>,
    departure: Option<&str>,
) -> transpo_rt::transit_realtime::trip_update::StopTimeUpdate {
    use transpo_rt::transit_realtime::*;

    trip_update::StopTimeUpdate {
        stop_sequence: Some(stop_sequence),
        stop_id: Some(stop_name.to_string()),
        arrival: Some(trip_update::StopTimeEvent {
            time: arrival.map(to_timestamp),
            ..Default::default()
        }),
        departure: Some(trip_update::StopTimeEvent {
            time: departure.map(to_timestamp),
            ..Default::default()
        }),
        schedule_relationship: None,
    }
}
