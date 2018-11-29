extern crate actix_web;
extern crate serde_json;
extern crate transpo_rt;
use actix_web::http;
use actix_web::test::TestServer;
use actix_web::HttpMessage;
use std::path::PathBuf;
use transpo_rt::stoppoints_discovery::Siri;

#[test]
fn sp_discovery_integration_test() {
    let make_server =
        || transpo_rt::server::create_server(&PathBuf::from("fixtures/gtfs.zip"), "".into());

    let mut srv = TestServer::with_factory(make_server);

    let request = srv
        .client(http::Method::GET, "/stoppoints_discovery.json")
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let siri: Siri = serde_json::from_str(body).unwrap();
    assert_eq!(siri.stop_points_delivery.version, "2.0");
    assert_eq!(siri.stop_points_delivery.status, true);
    // no filtering, we fetch all stops
    assert_eq!(siri.stop_points_delivery.annotated_stop_point.len(), 5);

    let stop1 = siri
        .stop_points_delivery
        .annotated_stop_point
        .iter()
        .find(|s| s.stop_point_ref == "stop2")
        .unwrap();

    assert_eq!(stop1.stop_name, "StopPoint");
    assert_eq!(stop1.location.longitude, 2.449386);
    assert_eq!(stop1.location.latitude, 48.796058);
    assert_eq!(stop1.lines.len(), 1);
    assert_eq!(stop1.lines[0].line_ref, "route1");
    //TODO more tests
}
