use crate::context::{Context, Data};
use crate::siri_model as model;
use actix_web::{error, Json, Query, Result, State};
use chrono::Timelike;
use gtfs_structures;
use serde;
use std::sync::Arc;

pub fn siri_datetime_param<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<chrono::Local>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let s = String::deserialize(deserializer)?;

    chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(&s)
        .map_err(serde::de::Error::custom)
        .map(|dt| dt.with_timezone(&chrono::Local))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Params {
    _requestor_ref: Option<String>,
    monitoring_ref: String,
    _line_ref: Option<String>,
    _destination_ref: Option<String>,
    #[serde(
        default = "chrono::Local::now",
        deserialize_with = "siri_datetime_param"
    )]
    start_time: chrono::DateTime<chrono::Local>,
    #[serde(skip)] //TODO
    _preview_interval: Option<chrono::Duration>,
}

// TODO move this helper
fn trip_is_valid(_trip: &gtfs_structures::Trip, _dt: &chrono::DateTime<chrono::Local>) -> bool {
    // TODO check the validity pattern
    true
}

fn make_dt(stop_time: u32) -> model::DateTime {
    let date = chrono::Local::now()
        .with_hour(stop_time / 60 / 60)
        .and_then(|d| d.with_minute(stop_time / 60 % 60))
        .and_then(|d| d.with_second(stop_time % 60))
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "".into());

    model::DateTime(date)
}

fn create_monitored_stop_visit(
    data: &Data,
    trip: &gtfs_structures::Trip,
    stop_time: &gtfs_structures::StopTime,
) -> model::MonitoredStopVisit {
    let call = model::MonitoredCall {
        order: stop_time.stop_sequence,
        stop_point_name: stop_time.stop.name.clone(),
        vehicle_at_stop: None,
        destination_display: None,
        aimed_arrival_time: Some(make_dt(stop_time.arrival_time)),
        aimed_departure_time: Some(make_dt(stop_time.departure_time)),
        expected_arrival_time: None,
        expected_departure_time: None,
    };
    model::MonitoredStopVisit {
        monitoring_ref: stop_time.stop.id.clone(),
        monitoring_vehicle_journey: model::MonitoredVehicleJourney {
            line_ref: data
                .gtfs
                .routes
                .get(&trip.route_id)
                .map(|r| r.id.clone())
                .unwrap_or_else(|| "line_unknown".into()),
            operator_ref: None,
            journey_pattern_ref: None,
            monitored_call: Some(call),
        },
        recorded_at_time: "".into(),
        item_identifier: "".into(),
    }
}

fn keep_stop_time(stop_time: &gtfs_structures::StopTime, request: &Params) -> bool {
    let request_seconds_since_midnight = {
        let dt = request.start_time;
        60 * 60 * dt.hour() + 60 * dt.minute() + dt.second()
    };
    // TODO check request.preview_interval
    stop_time.departure_time >= request_seconds_since_midnight
}

fn create_stop_monitoring(
    stop: &Arc<gtfs_structures::Stop>,
    data: &Data,
    request: &Params,
) -> Vec<model::StopMonitoringDelivery> {
    let mut stop_times = Vec::new(); //TODO rustify all this....
    for trip in data
        .gtfs
        .trips
        .values()
        .filter(|t| trip_is_valid(t, &request.start_time))
    {
        for st in trip
            .stop_times
            .iter()
            .filter(|stop_time| {
                let stop_id = &stop_time.stop.id;
                stop_id == &stop.id || stop.parent_station.as_ref() == Some(stop_id)
            })
            .filter(|stop_time| keep_stop_time(&stop_time, request))
        // TODO filter on departure after request.start_time
        // TODO filter on the other request's param (PreviewInterval, MaximumStopVisits)
        {
            stop_times.push((trip, st));
        }
    }

    stop_times.sort_by_key(|s| s.1.departure_time);

    vec![model::StopMonitoringDelivery {
        monitored_stop_visits: stop_times
            .into_iter()
            .map(|(trip, st)| create_monitored_stop_visit(data, trip, st))
            .take(2)
            .collect(),
    }]
}

pub fn stop_monitoring(
    (state, query): (State<Context>, Query<Params>),
) -> Result<Json<model::SiriResponse>> {
    let arc_data = state.data.clone();

    let data = arc_data.lock().unwrap();
    let stops = &data.gtfs.stops;

    let request = query.into_inner();

    let stop = stops.get(&request.monitoring_ref).ok_or_else(|| {
        error::ErrorNotFound(format!(
            "impossible to find stop: '{}'",
            &request.monitoring_ref
        ))
    })?;

    Ok(Json(model::SiriResponse {
        siri: model::Siri {
            service_delivery: Some(model::ServiceDelivery {
                response_time_stamp: chrono::Local::now().to_rfc3339(),
                producer_ref: "".into(),
                address: "".into(),
                response_message_identifier: "".into(),
                request_message_ref: "".into(),
                stop_monitoring_delivery: create_stop_monitoring(&stop, &data, &request),
            }),
            ..Default::default()
        },
    }))
}
