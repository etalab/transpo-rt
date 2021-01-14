use actix_web::http::StatusCode;
use transpo_rt::datasets::DatasetInfo;
mod utils;

#[actix_rt::test]
async fn start_with_invalid_gtfs_test() {
    let _log_guard = utils::init_log();

    let mut srv = utils::make_test_server(vec![DatasetInfo::new_default(
        "fixtures/this_file_does_not_exist.zip",
        &[mockito::server_url() + "/gtfs_rt_1"],
    )])
    .await;

    assert_eq!(
        utils::get_status(&mut srv, "/default").await,
        StatusCode::BAD_GATEWAY
    );
    //assert_eq!(utils::get_status(&mut srv, "/default/gtfs-rt.json").await, StatusCode::OK);
    //assert_eq!(utils::get_status(&mut srv, "/default/gtfs-rt").await, StatusCode::OK);
    assert_eq!(
        utils::get_status(&mut srv, "/default/siri/2.0/stop-monitoring.json").await,
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        utils::get_status(&mut srv, "/default/siri/2.0/stoppoints-discovery.json").await,
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        utils::get_status(&mut srv, "/default/siri/2.0/general-message.json").await,
        StatusCode::BAD_GATEWAY
    );
}
