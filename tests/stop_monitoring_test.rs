use actix_web::http;
use actix_web::HttpMessage;
use transpo_rt::siri_model::SiriResponse;

mod utils;

#[test]
fn sp_monitoring_integration_test() {
    let mut srv = utils::make_test_server();

    let request = srv
        .client(
            http::Method::GET,
            "/stop_monitoring.json?MonitoringRef=EMSI&StartTime=2018-12-15T15:22:00&DataFreshness=Scheduled",
        )
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();

    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let resp: SiriResponse = serde_json::from_str(body).unwrap();
    let spd = resp.siri.service_delivery.unwrap();
    let sm = spd.stop_monitoring_delivery.iter().next().unwrap();

    assert_eq!(sm.monitored_stop_visits.len(), 2);

    let first_passage = &sm.monitored_stop_visits[0];

    assert_eq!(first_passage.monitoring_ref, "EMSI");
    let vj = &first_passage.monitoring_vehicle_journey;
    assert_eq!(vj.line_ref, "CITY");
    let passage = &vj.monitored_call.as_ref().unwrap();
    assert_eq!(
        passage.aimed_arrival_time.as_ref().map(|t| t.to_string()),
        Some("2018-12-15T15:26:00".into())
    );
    assert_eq!(
        passage.aimed_departure_time.as_ref().map(|t| t.to_string()),
        Some("2018-12-15T15:28:00".into())
    );
    assert_eq!(passage.order, 5);
    assert_eq!(passage.stop_point_name, "E Main St / S Irving St (Demo)");
}
