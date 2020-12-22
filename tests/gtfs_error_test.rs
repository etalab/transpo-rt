use actix_web::http::StatusCode;
use transpo_rt::datasets::DatasetInfo;
mod utils;

#[actix_rt::test]
async fn start_with_invalid_gtfs_test() {
    let _log_guard = utils::init_log();

    let srv = utils::make_test_server(vec![DatasetInfo::new_default(
        "fixtures/invalid_gtfs.zip",
        &[mockito::server_url() + "/gtfs_rt_1"],
    )])
    .await;

    let response = srv.get("/default/").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}
