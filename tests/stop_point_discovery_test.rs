use actix_web::http;
use actix_web::HttpMessage;
use std::collections::BTreeSet;
use transpo_rt::siri_lite::SiriResponse;

mod utils;

#[test]
fn sp_discovery_integration_test() {
    let _log_guard = utils::init_log();
    let mut srv = utils::make_simple_test_server();

    let request = srv
        .client(
            http::Method::GET,
            "/default/siri/2.0/stoppoints-discovery.json?q=mai",
        )
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();

    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let resp: SiriResponse = serde_json::from_str(body).unwrap();
    let spd = resp.siri.stop_points_delivery.unwrap();
    assert_eq!(spd.common.version, "2.0");
    assert_eq!(spd.common.status, Some(true));
    // no filtering, we fetch all stops
    assert_eq!(spd.annotated_stop_point.len(), 1);

    let stop1 = spd
        .annotated_stop_point
        .iter()
        .find(|s| s.stop_point_ref == "EMSI")
        .unwrap();

    assert_eq!(stop1.stop_name, "E Main St / S Irving St (Demo)");
    assert_eq!(stop1.location.longitude, -116.76218);
    assert_eq!(stop1.location.latitude, 36.905697);
    assert_eq!(
        stop1
            .lines
            .iter()
            .map(|l| l.line_ref.clone())
            .collect::<BTreeSet<_>>(),
        vec!["CITY_R".into(), "CITY".into()].into_iter().collect()
    );
    //TODO more tests
}
