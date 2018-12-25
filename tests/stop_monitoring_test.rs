use actix_web::http;
use actix_web::HttpMessage;
use transpo_rt::siri_model::SiriResponse;
use transpo_rt::transit_realtime;
mod utils;

#[test]
fn sp_monitoring_integration_test() {
    let mut srv = utils::make_test_server();

    let request = srv
        .client(
            http::Method::GET,
            "/stop_monitoring.json?MonitoringRef=EMSI&StartTime=2018-12-15T05:22:00&DataFreshness=Scheduled",
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
        Some("2018-12-15T06:26:00".into())
    );
    assert_eq!(
        passage.aimed_departure_time.as_ref().map(|t| t.to_string()),
        Some("2018-12-15T06:28:00".into())
    );
    assert_eq!(passage.order, 5);
    assert_eq!(passage.stop_point_name, "E Main St / S Irving St (Demo)");
}

// take a date (formated as YYYY-MM-DDTHH:MM:SS) and convert it to a timestamp
fn to_timestamp(date: &str) -> i64 {
    chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S")
        .expect("impossible to parse datetime")
        .timestamp()
}

fn make_stu(
    stop_name: &str,
    stop_sequence: u32,
    arrival: Option<&str>,
    departure: Option<&str>,
) -> transpo_rt::transit_realtime::trip_update::StopTimeUpdate {
    use transpo_rt::transit_realtime::*;

    trip_update::StopTimeUpdate {
        stop_sequence: Some(stop_sequence),
        stop_id: Some(stop_name.to_string()),
        arrival: Some(trip_update::StopTimeEvent {
            time: arrival.map(to_timestamp),
            ..Default::default()
        }),
        departure: Some(trip_update::StopTimeEvent {
            time: departure.map(to_timestamp),
            ..Default::default()
        }),
        schedule_relationship: None,
    }
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
                stop_time_update: vec![make_stu(
                    "EMSI",
                    5,
                    // Some("2018-12-15T6:26:30"),
                    // Some("2018-12-15T6:28:30"),
                    Some("2018-12-15T5:26:30"), //ugly hack, for the moment the date is set one hour before the right time
                    Some("2018-12-15T5:28:30"), // this will change after the timezone are correctly handled
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
    let mut srv = utils::make_test_server();

    let gtfs_rt = create_mock_feed_message();
    let _server = utils::run_simple_gtfs_rt_server(gtfs_rt);
    let request = srv
        .client(
            http::Method::GET,
            "/stop_monitoring.json?MonitoringRef=EMSI&StartTime=2018-12-15T05:22:00",
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
        Some("2018-12-15T06:26:00".into())
    );
    assert_eq!(
        passage
            .expected_arrival_time
            .as_ref()
            .map(|t| t.to_string()),
        Some("2018-12-15T06:26:30".into())
    );
    assert_eq!(
        passage.aimed_departure_time.as_ref().map(|t| t.to_string()),
        Some("2018-12-15T06:28:00".into())
    );
    assert_eq!(
        passage
            .expected_departure_time
            .as_ref()
            .map(|t| t.to_string()),
        Some("2018-12-15T06:28:30".into())
    );
    assert_eq!(passage.order, 5);
    assert_eq!(passage.stop_point_name, "E Main St / S Irving St (Demo)");
}
