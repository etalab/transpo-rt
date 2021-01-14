use actix_web::http::StatusCode;
use transpo_rt::datasets::DatasetInfo;
use transpo_rt::transit_realtime;
mod utils;

fn create_mock_feed_message() -> transit_realtime::FeedMessage {
    use transpo_rt::transit_realtime::*;
    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".into(),
            incrementality: Some(0i32),
            timestamp: Some(1u64),
        },
        entity: vec![],
    }
}

#[actix_rt::test]
async fn start_with_invalid_gtfs_test() {
    let _log_guard = utils::init_log();

    let gtfs_rt = create_mock_feed_message();
    let _server = utils::run_simple_gtfs_rt_server(gtfs_rt);
    let mut srv = utils::make_test_server(vec![DatasetInfo::new_default(
        "fixtures/this_file_does_not_exist.zip",
        &[mockito::server_url() + "/gtfs_rt"],
    )])
    .await;

    assert_eq!(
        utils::get_status(&mut srv, "/default").await,
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        utils::get_status(&mut srv, "/default/gtfs-rt.json/").await,
        StatusCode::OK
    );
    assert_eq!(
        utils::get_status(&mut srv, "/default/gtfs-rt").await,
        StatusCode::OK
    );
    assert_eq!(
        utils::get_status(
            &mut srv,
            "/default/siri/2.0/stop-monitoring.json?MonitoringRef=some_code"
        )
        .await,
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        utils::get_status(
            &mut srv,
            "/default/siri/2.0/stoppoints-discovery.json?q=some_query"
        )
        .await,
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        utils::get_status(&mut srv, "/default/siri/2.0/general-message.json").await,
        StatusCode::BAD_GATEWAY
    );
}
