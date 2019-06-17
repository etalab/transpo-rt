use actix_web::http;
use actix_web::HttpMessage;
use transpo_rt::siri_lite::{DateTime, SiriResponse};
use transpo_rt::transit_realtime;
mod utils;

fn string(time: &Option<DateTime>) -> Option<String> {
    time.as_ref().map(|t| t.to_string())
}

#[test]
fn sp_monitoring_integration_test() {
    let _log_guard = utils::init_log();
    let mut srv = utils::make_simple_test_server();

    let request = srv
        .client(
            http::Method::GET,
            r#"/default/siri/2.0/stop-monitoring.json?
MonitoringRef=EMSI&
StartTime=2018-12-15T05:22:00&
DataFreshness=Scheduled&
MaximumStopVisits=3"#,
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

    assert_eq!(sm.monitored_stop_visit.len(), 3);

    let first_passage = &sm.monitored_stop_visit[0];

    assert_eq!(first_passage.monitoring_ref, "EMSI");
    let vj = &first_passage.monitored_vehicle_journey;
    assert_eq!(vj.line_ref, "CITY");
    assert_eq!(vj.service_info.operator_ref, Some("DTA".to_owned()));
    let passage = &vj.monitored_call.as_ref().unwrap();
    assert_eq!(
        string(&passage.aimed_arrival_time),
        Some("2018-12-15T06:26:00".into())
    );
    assert!(passage.expected_arrival_time.is_none());
    assert_eq!(
        string(&passage.aimed_departure_time),
        Some("2018-12-15T06:28:00".into())
    );
    assert!(passage.expected_departure_time.is_none());
    assert_eq!(passage.order, 5);
    assert_eq!(passage.stop_point_name, "E Main St / S Irving St (Demo)");

    // Note: to reduce the number of time the dataset is loaded (thus the integration tests running time)
    // we chain some different tests
    test_interval_filtering(&mut srv);
    test_beatty_stop_call(&mut srv);
}

// test stop_monitoring on BEATTY_AIRPORT
// multiple lines pass though this stop so we should be able to test more stuff
fn test_beatty_stop_call(srv: &mut actix_web::test::TestServer) {
    let request = srv
        .client(
            http::Method::GET,
            "/default/siri/2.0/stop-monitoring.json?MonitoringRef=BEATTY_AIRPORT&StartTime=2018-12-15T05:22:00&DataFreshness=Scheduled",
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

    assert_eq!(sm.monitored_stop_visit.len(), 2);

    // there should be a passage at 6:20 for line STBA
    // and a passage at 08:00 for line AB

    let first_passage = &sm.monitored_stop_visit[0];
    assert_eq!(first_passage.monitoring_ref, "BEATTY_AIRPORT");
    assert_eq!(first_passage.item_identifier, "BEATTY_AIRPORT:STBA");
    let vj = &first_passage.monitored_vehicle_journey;
    assert_eq!(vj.line_ref, "STBA");
    let first_passage = &vj.monitored_call.as_ref().unwrap();
    assert_eq!(
        string(&first_passage.aimed_arrival_time),
        Some("2018-12-15T06:20:00".to_owned())
    );
    assert_eq!(
        string(&first_passage.aimed_departure_time),
        Some("2018-12-15T06:20:00".to_owned())
    );
    assert!(first_passage.expected_arrival_time.is_none());
    assert!(first_passage.expected_departure_time.is_none());
    assert_eq!(first_passage.order, 2);

    // second passage on line AB
    let second_passage = &sm.monitored_stop_visit[1];

    assert_eq!(second_passage.monitoring_ref, "BEATTY_AIRPORT");
    assert_eq!(second_passage.item_identifier, "BEATTY_AIRPORT:AB1");
    let vj = &second_passage.monitored_vehicle_journey;
    assert_eq!(vj.line_ref, "AB");
    let second_passage = &vj.monitored_call.as_ref().unwrap();
    assert_eq!(
        string(&second_passage.aimed_arrival_time),
        Some("2018-12-15T08:00:00".to_owned())
    );
    assert_eq!(
        string(&second_passage.aimed_departure_time),
        Some("2018-12-15T08:00:00".to_owned())
    );
    assert!(second_passage.expected_arrival_time.is_none());
    assert!(second_passage.expected_departure_time.is_none());
    assert_eq!(second_passage.order, 1);

    // now we do the same call, but we filter by line_ref=AB
    // we should have only passage for this line (and still 2 passages)
    let request = srv
        .client(
            http::Method::GET,
            "/default/siri/2.0/stop-monitoring.json?MonitoringRef=BEATTY_AIRPORT&StartTime=2018-12-15T05:22:00&DataFreshness=Scheduled&LineRef=AB",
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

    assert_eq!(sm.monitored_stop_visit.len(), 2);

    let first_passage = &sm.monitored_stop_visit[0];
    assert_eq!(first_passage.monitoring_ref, "BEATTY_AIRPORT");
    assert_eq!(first_passage.item_identifier, "BEATTY_AIRPORT:AB1");
    let vj = &first_passage.monitored_vehicle_journey;
    assert_eq!(vj.line_ref, "AB");
    let first_passage = &vj.monitored_call.as_ref().unwrap();
    assert_eq!(
        string(&first_passage.aimed_arrival_time),
        Some("2018-12-15T08:00:00".to_owned())
    );
    assert_eq!(
        string(&first_passage.aimed_departure_time),
        Some("2018-12-15T08:00:00".to_owned())
    );
    assert!(first_passage.expected_arrival_time.is_none());
    assert!(first_passage.expected_departure_time.is_none());
    assert_eq!(first_passage.order, 1);

    let second_passage = &sm.monitored_stop_visit[1];
    assert_eq!(second_passage.monitoring_ref, "BEATTY_AIRPORT");
    assert_eq!(second_passage.item_identifier, "BEATTY_AIRPORT:AB2");
    let vj = &second_passage.monitored_vehicle_journey;
    assert_eq!(vj.line_ref, "AB");
    let second_passage = &vj.monitored_call.as_ref().unwrap();
    assert_eq!(
        string(&second_passage.aimed_arrival_time),
        Some("2018-12-15T12:15:00".to_owned())
    );
    assert_eq!(
        string(&second_passage.aimed_departure_time),
        Some("2018-12-15T12:15:00".to_owned())
    );
    assert!(second_passage.expected_arrival_time.is_none());
    assert!(second_passage.expected_departure_time.is_none());
    assert_eq!(second_passage.order, 2);
}

// we filter the departure/arrival within the hour, we should have only 1 departure
// Note: since it is not specified in the spec, we filter on the scheduled departure/arrival time
fn test_interval_filtering(srv: &mut actix_web::test::TestServer) {
    let request = srv
        .client(
            http::Method::GET,
            r#"/default/siri/2.0/stop-monitoring.json?
MonitoringRef=BEATTY_AIRPORT&
StartTime=2018-12-15T05:22:00&
DataFreshness=Scheduled
&PreviewInterval=PT1H"#,
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

    assert_eq!(sm.monitored_stop_visit.len(), 1);
}

fn create_mock_feed_message() -> transit_realtime::FeedMessage {
    use transpo_rt::transit_realtime::*;
    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".into(),
            incrementality: Some(0i32),
            timestamp: Some(1u64),
        },
        entity: vec![FeedEntity {
            id: "delay_on_city1".into(),
            trip_update: Some(TripUpdate {
                trip: TripDescriptor {
                    trip_id: Some("CITY1".into()),
                    start_date: Some("20181215".into()),
                    ..Default::default()
                },
                stop_time_update: vec![utils::make_stu(
                    "EMSI",
                    5,
                    Some("2018-12-15T06:26:30-08:00"),
                    Some("2018-12-15T06:28:30-08:00"),
                )],
                ..Default::default()
            }),
            ..Default::default()
        }],
    }
}

// integration test for stop_monitoring
// we query the same stop as sp_monitoring_integration_test ("EMSI")
// but we mock a gtfs_rt saying that the bus will be 30s late
#[test]
fn sp_monitoring_relatime_integration_test() {
    let _log_guard = utils::init_log();
    let gtfs_rt = create_mock_feed_message();
    let _server = utils::run_simple_gtfs_rt_server(gtfs_rt);

    let mut srv = utils::make_simple_test_server();

    let request = srv
        .client(
            http::Method::GET,
            "/default/siri/2.0/stop-monitoring.json?MonitoringRef=EMSI&StartTime=2018-12-15T05:22:00",
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

    assert_eq!(sm.monitored_stop_visit.len(), 2);

    let first_passage = &sm.monitored_stop_visit[0];

    assert_eq!(first_passage.monitoring_ref, "EMSI");
    let vj = &first_passage.monitored_vehicle_journey;
    assert_eq!(vj.line_ref, "CITY");
    let passage = &vj.monitored_call.as_ref().unwrap();
    assert_eq!(
        string(&passage.aimed_arrival_time),
        Some("2018-12-15T06:26:00".into())
    );
    assert_eq!(
        string(&passage.expected_arrival_time),
        Some("2018-12-15T06:26:30".into())
    );
    assert_eq!(
        string(&passage.aimed_departure_time),
        Some("2018-12-15T06:28:00".into())
    );
    assert_eq!(
        string(&passage.expected_departure_time),
        Some("2018-12-15T06:28:30".into())
    );
    assert_eq!(passage.order, 5);
    assert_eq!(passage.stop_point_name, "E Main St / S Irving St (Demo)");
}
