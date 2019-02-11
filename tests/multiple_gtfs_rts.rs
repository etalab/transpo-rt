use actix_web::http;
use actix_web::HttpMessage;
use transpo_rt::datasets::DatasetInfo;
use transpo_rt::siri_model::{DateTime, SiriResponse};
use transpo_rt::transit_realtime;
mod utils;

fn string(time: &Option<DateTime>) -> Option<String> {
    time.as_ref().map(|t| t.to_string())
}

fn create_mock_feed_message_stba() -> transit_realtime::FeedMessage {
    use transpo_rt::transit_realtime::*;
    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".into(),
            incrementality: Some(0i32),
            timestamp: Some(1u64),
        },
        entity: vec![FeedEntity {
            id: "delay_on_stba".into(),
            trip_update: Some(TripUpdate {
                trip: TripDescriptor {
                    trip_id: Some("STBA".into()),
                    start_date: Some("20181215".into()),
                    ..Default::default()
                },
                stop_time_update: vec![utils::make_stu(
                    "BEATTY_AIRPORT",
                    2,
                    Some("2018-12-15T06:26:30-08:00"),
                    Some("2018-12-15T06:28:31-08:00"),
                )],
                ..Default::default()
            }),
            ..Default::default()
        }],
    }
}

fn create_mock_feed_message_ab() -> transit_realtime::FeedMessage {
    use transpo_rt::transit_realtime::*;
    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".into(),
            incrementality: Some(0i32),
            timestamp: Some(1u64),
        },
        entity: vec![FeedEntity {
            id: "delay_on_ab".into(),
            trip_update: Some(TripUpdate {
                trip: TripDescriptor {
                    trip_id: Some("AB1".into()),
                    start_date: Some("20181215".into()),
                    ..Default::default()
                },
                stop_time_update: vec![utils::make_stu(
                    "BEATTY_AIRPORT",
                    1,
                    Some("2018-12-15T08:28:30-08:00"),
                    Some("2018-12-15T08:28:31-08:00"),
                )],
                ..Default::default()
            }),
            ..Default::default()
        }],
    }
}

/// Integration tests with multiple GTFS_RT
/// there is one gtfs_rt server that provides a delay on the line STBA
/// and another one that provides a delay on AB
/// when querying /stop_monitoring we should have the 2 delays
/// we also test that the resulting GTFS_RT provided by /gtfs_rt is valid
#[test]
fn multiple_gtfs_rt_integration_test() {
    let gtfs_rt1 = create_mock_feed_message_stba();
    let gtfs_rt2 = create_mock_feed_message_ab();
    let _server1 = utils::run_gtfs_rt_server("/gtfs_rt_1", gtfs_rt1);
    let _server2 = utils::run_gtfs_rt_server("/gtfs_rt_2", gtfs_rt2);

    let mut srv = utils::make_test_server(vec![DatasetInfo::new_default(
        "fixtures/gtfs.zip",
        &[
            mockito::server_url().to_string() + "/gtfs_rt_1",
            mockito::server_url().to_string() + "/gtfs_rt_2",
        ],
    )]);

    test_stop_monitoring(&mut srv);
    test_gtfs_rt(&mut srv);
}

fn test_stop_monitoring(srv: &mut actix_web::test::TestServer) {
    let request = srv
        .client(
            http::Method::GET,
            "/default/siri-lite/stop_monitoring.json?MonitoringRef=BEATTY_AIRPORT&StartTime=2018-12-15T05:22:00",
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
    assert_eq!(first_passage.monitoring_ref, "BEATTY_AIRPORT");
    assert_eq!(first_passage.item_identifier, "BEATTY_AIRPORT:STBA");
    let vj = &first_passage.monitoring_vehicle_journey;
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
    assert_eq!(
        string(&first_passage.expected_arrival_time),
        Some("2018-12-15T06:26:30".to_owned())
    );
    assert_eq!(
        string(&first_passage.expected_departure_time),
        Some("2018-12-15T06:28:31".to_owned())
    );
    assert_eq!(first_passage.order, 2);

    // second passage on line AB
    let second_passage = &sm.monitored_stop_visits[1];

    assert_eq!(second_passage.monitoring_ref, "BEATTY_AIRPORT");
    assert_eq!(second_passage.item_identifier, "BEATTY_AIRPORT:AB1");
    let vj = &second_passage.monitoring_vehicle_journey;
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
    assert_eq!(
        string(&second_passage.expected_arrival_time),
        Some("2018-12-15T08:28:30".to_owned())
    );
    assert_eq!(
        string(&second_passage.expected_departure_time),
        Some("2018-12-15T08:28:31".to_owned())
    );
    assert_eq!(second_passage.order, 1);
}

fn test_gtfs_rt(srv: &mut actix_web::test::TestServer) {
    let request = srv
        .client(http::Method::GET, "/default/gtfs_rt.json")
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let feed: transit_realtime::FeedMessage = serde_json::from_str(body).unwrap();

    // the resulting gtfs_rt should have both entities
    let entities: std::collections::BTreeSet<_> =
        feed.entity.iter().map(|e| e.id.as_str()).collect();
    let expected = maplit::btreeset! {"delay_on_stba", "delay_on_ab"};
    assert_eq!(entities, expected);
}
