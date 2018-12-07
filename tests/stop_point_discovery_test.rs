use actix_web::http;
use actix_web::test::TestServer;
use actix_web::HttpMessage;
use transpo_rt::siri_model::SiriResponse;

#[test]
fn sp_discovery_integration_test() {
    let make_server = || {
        transpo_rt::server::create_server(transpo_rt::server::make_context("fixtures/gtfs.zip", ""))
    };

    let mut srv = TestServer::with_factory(make_server);

    let request = srv
        .client(http::Method::GET, "/stoppoints_discovery.json")
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let resp: SiriResponse = serde_json::from_str(body).unwrap();
    let spd = resp.siri.stop_points_delivery.unwrap();
    assert_eq!(spd.version, "2.0");
    assert_eq!(spd.status, true);
    // no filtering, we fetch all stops
    assert_eq!(spd.annotated_stop_point.len(), 5);

    let stop1 = spd
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
