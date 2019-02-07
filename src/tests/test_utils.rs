use crate::transit_realtime;

// take a date (formated as YYYY-MM-DDTHH:MM:SS) and convert it to a timestamp
fn to_timestamp(date: &str) -> i64 {
    chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(date)
        .expect("impossible to parse datetime")
        .timestamp()
}

pub fn make_stu(
    stop_name: &str,
    stop_sequence: u32,
    arrival: Option<&str>,
    departure: Option<&str>,
) -> transit_realtime::trip_update::StopTimeUpdate {
    use transit_realtime::trip_update::*;

    StopTimeUpdate {
        stop_sequence: Some(stop_sequence),
        stop_id: Some(stop_name.to_string()),
        arrival: Some(StopTimeEvent {
            time: arrival.map(to_timestamp),
            ..Default::default()
        }),
        departure: Some(StopTimeEvent {
            time: departure.map(to_timestamp),
            ..Default::default()
        }),
        schedule_relationship: None,
    }
}

pub fn trip_update(id: &str, tu: transit_realtime::TripUpdate) -> transit_realtime::FeedEntity {
    transit_realtime::FeedEntity {
        id: id.to_owned(),
        trip_update: Some(tu),
        ..Default::default()
    }
}

pub fn create_feed_message(
    entities: &[transit_realtime::FeedEntity],
) -> transit_realtime::FeedMessage {
    use transit_realtime::*;
    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".into(),
            incrementality: Some(0i32),
            timestamp: Some(1u64),
        },
        entity: entities.to_vec(),
    }
}
