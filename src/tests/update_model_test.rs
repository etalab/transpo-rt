use crate::model_update;
use crate::tests::test_utils::{create_feed_message, make_stu, trip_update};
use crate::transit_realtime as tr;
use chrono::NaiveDateTime;
use model_builder::ModelBuilder;
use std::str::FromStr;

fn ndt(d: &str) -> NaiveDateTime {
    NaiveDateTime::from_str(d).unwrap()
}

fn simple_dataset() -> navitia_model::Model {
    ModelBuilder::default()
        .vj("vj1", |vj_builder| {
            vj_builder
                .st("A", "10:00:00", "10:01:00")
                .st("B", "11:00:00", "11:01:00")
                .st("C", "12:00:00", "12:01:00")
                .st("D", "13:00:00", "13:01:00")
                .st("E", "14:00:00", "14:01:00");
        })
        .build()
}

fn create_simple_gtfs_rt() -> tr::FeedMessage {
    create_feed_message(&[trip_update(
        "delay_trip_A",
        tr::TripUpdate {
            trip: tr::TripDescriptor {
                trip_id: Some("vj1".to_owned()),
                start_date: Some("20181215".to_owned()),
                ..Default::default()
            },
            stop_time_update: vec![
                make_stu(
                    "B",
                    2,
                    Some("2018-12-15T11:00:30Z"),
                    Some("2018-12-15T11:01:30Z"),
                ),
                make_stu(
                    "D",
                    4,
                    Some("2018-12-15T13:00:30Z"),
                    Some("2018-12-15T13:01:30Z"),
                ),
            ],
            ..Default::default()
        },
    )])
}

#[test]
fn read_simple_gtfs_rt() {
    let model = simple_dataset();
    let gtfs_rt = create_simple_gtfs_rt();

    let model_update = model_update::get_model_update(&model, &[gtfs_rt], chrono_tz::UTC).unwrap();

    assert_eq!(model_update.trips.len(), 1);

    let dated_vj = crate::datasets::DatedVehicleJourney {
        vj_idx: model.vehicle_journeys.get_idx("vj1").unwrap(),
        date: chrono::NaiveDate::from_ymd(2018, 12, 15),
    };
    let trip_update = &model_update.trips[&dated_vj];

    let stu = &trip_update.stop_time_update_by_sequence;

    assert_eq!(stu.len(), 2);
    // TODO: The real gtfs_rt specification would require this to be 4 (B -> C -> D -> E)
    // but for the moment we don't implement holes (C) and extensions (E)
    // assert_eq!(stu.len(), 4);
    assert_eq!(
        stu[&2],
        model_update::StopTimeUpdate {
            stop_point_idx: model.stop_points.get_idx("B").unwrap(),
            updated_arrival: Some(ndt("2018-12-15T11:00:30")),
            updated_departure: Some(ndt("2018-12-15T11:01:30")),
        }
    );
    assert_eq!(
        stu[&4],
        model_update::StopTimeUpdate {
            stop_point_idx: model.stop_points.get_idx("D").unwrap(),
            updated_arrival: Some(ndt("2018-12-15T13:00:30")),
            updated_departure: Some(ndt("2018-12-15T13:01:30")),
        }
    );
}

#[test]
fn feed_on_unknown_stop_and_trip() {
    let model = simple_dataset();
    let gtfs_rt = create_feed_message(&[
        trip_update(
            "delay_trip_A",
            tr::TripUpdate {
                trip: tr::TripDescriptor {
                    trip_id: Some("vj1".to_owned()),
                    start_date: Some("20181215".to_owned()),
                    ..Default::default()
                },
                stop_time_update: vec![
                    make_stu(
                        "B",
                        2,
                        Some("2018-12-15T11:00:30Z"),
                        Some("2018-12-15T11:01:30Z"),
                    ),
                    make_stu(
                        "invalid_stop",
                        4,
                        Some("2018-12-15T13:00:30Z"),
                        Some("2018-12-15T13:01:30Z"),
                    ),
                    make_stu(
                        "D",
                        4,
                        Some("2018-12-15T14:00:30Z"),
                        None, // None is still valid
                    ),
                ],
                ..Default::default()
            },
        ),
        trip_update(
            "invalid_trip_message",
            tr::TripUpdate {
                trip: tr::TripDescriptor {
                    trip_id: Some("invalid_trip".to_owned()),
                    start_date: Some("20181215".to_owned()),
                    ..Default::default()
                },
                stop_time_update: vec![make_stu(
                    "B",
                    2,
                    Some("2018-12-15T11:00:30Z"),
                    Some("2018-12-15T11:01:30Z"),
                )],
                ..Default::default()
            },
        ),
    ]);

    let model_update = model_update::get_model_update(&model, &[gtfs_rt], chrono_tz::UTC).unwrap();

    // we should have only 1 trip_update on the 2 from the feed, because one of them is invalid (on an invalid vj)
    assert_eq!(model_update.trips.len(), 1);

    let dated_vj = crate::datasets::DatedVehicleJourney {
        vj_idx: model.vehicle_journeys.get_idx("vj1").unwrap(),
        date: chrono::NaiveDate::from_ymd(2018, 12, 15),
    };
    let trip_update = &model_update.trips[&dated_vj];

    let stu = &trip_update.stop_time_update_by_sequence;

    // of the 3 feed's stoptime's update, only 2 are valid, the one on invalid stop should have been skiped
    assert_eq!(stu.len(), 2);
    assert_eq!(
        stu[&2],
        model_update::StopTimeUpdate {
            stop_point_idx: model.stop_points.get_idx("B").unwrap(),
            updated_arrival: Some(ndt("2018-12-15T11:00:30")),
            updated_departure: Some(ndt("2018-12-15T11:01:30")),
        }
    );
    assert_eq!(
        stu[&4],
        model_update::StopTimeUpdate {
            stop_point_idx: model.stop_points.get_idx("D").unwrap(),
            updated_arrival: Some(ndt("2018-12-15T14:00:30")),
            updated_departure: None,
        }
    );
}
