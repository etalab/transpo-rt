use actix_web::http;
use actix_web::HttpMessage;
use transpo_rt::datasets::DatasetInfo;
mod utils;

#[test]
fn list_datasets_integration_test() {
    let mut srv = utils::make_simple_test_server();

    let request = srv.client(http::Method::GET, "/").finish().unwrap();
    let response = srv.execute(request.send()).unwrap();

    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let datasets: Vec<DatasetInfo> = serde_json::from_str(&body).unwrap();

    assert_eq!(datasets.len(), 1);
    let dataset = &datasets[0];
    assert_eq!(dataset.id, "default");
    assert_eq!(dataset.name, "default name");
    assert_eq!(dataset.gtfs, "fixtures/gtfs.zip");
    assert!(!dataset.gtfs_rt_urls.is_empty());
}
