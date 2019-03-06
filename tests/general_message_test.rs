use actix_web::http;
use actix_web::HttpMessage;
use transpo_rt::transit_realtime;
mod utils;

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
            alert: Some(Alert {
                informed_entity: vec![
                    EntitySelector {
                        route_id: Some("route_1".to_owned()),
                        ..Default::default()
                    },
                    EntitySelector {
                        stop_id: Some("stop_1".to_owned()),
                        ..Default::default()
                    },
                ],
                header_text: Some(TranslatedString {
                    translation: vec![
                        translated_string::Translation {
                            text: "huge problem".to_owned(),
                            language: None,
                        },
                        translated_string::Translation {
                            text: "gros probleme".to_owned(),
                            language: Some("fr".to_owned()),
                        },
                    ],
                }),
                description_text: Some(TranslatedString {
                    translation: vec![translated_string::Translation {
                        text: "huge problem on the route 1 and stop 1".to_owned(),
                        language: None,
                    }],
                }),
                active_period: vec![TimeRange {
                    start: Some(utils::to_timestamp("2018-12-15T08:00:00-08:00") as u64),
                    end: Some(utils::to_timestamp("2018-12-15T12:00:00-08:00") as u64),
                }],
                ..Default::default()
            }),
            ..Default::default()
        }],
    }
}

#[test]
fn general_message_integration_test() {
    let gtfs_rt = create_mock_feed_message();
    let _server = utils::run_simple_gtfs_rt_server(gtfs_rt);

    let mut srv = utils::make_simple_test_server();

    call_in_activity_period(&mut srv);
    call_not_in_activity_period(&mut srv);
}

fn call_in_activity_period(srv: &mut actix_web::test::TestServer) {
    let request = srv
        .client(
            http::Method::GET,
            "/default/siri/2.0/general-message.json?RequestTimestamp=2018-12-15T10:00:00",
        )
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();

    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let resp: serde_json::Value = serde_json::from_str(body).unwrap();

    let messages = resp.pointer("/Siri/ServiceDelivery/GeneralMessageDelivery/0/InfoMessages");

    assert_eq!(
        messages,
        Some(&serde_json::json! ([
          {
            "Content": {
              "LineRef": [
                "route_1"
              ],
              "StopPointRef": [
                "stop_1"
              ],
              "Message": [
                {
                  "MessageType": "shortMessage",
                  "MessageText": {
                    "Value": "huge problem"
                  }
                },
                {
                  "MessageType": "shortMessage",
                  "MessageText": {
                    "Lang": "fr",
                    "Value": "gros probleme"
                  }
                },
                {
                  "MessageType": "longMessage",
                  "MessageText": {
                    "Value": "huge problem on the route 1 and stop 1"
                  }
                }
              ]
            },
            "ValidUntilTime": "2018-12-15T12:00:00"
          }
        ]))
    );
}

fn call_not_in_activity_period(srv: &mut actix_web::test::TestServer) {
    let request = srv
        .client(
            http::Method::GET,
            "/default/siri/2.0/general-message.json?RequestTimestamp=2018-12-15T14:00:00",
        )
        .finish()
        .unwrap();
    let response = srv.execute(request.send()).unwrap();

    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let resp: serde_json::Value = serde_json::from_str(body).unwrap();

    let messages = resp.pointer("/Siri/ServiceDelivery/GeneralMessageDelivery/0/InfoMessages");

    // we are outside the activity period, we should not have any msg
    assert_eq!(messages, Some(&serde_json::json!([])));
}
