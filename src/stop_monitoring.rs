use crate::context::{Connection, Context, Data, RealTimeConnection, ScheduleRelationship};
use crate::gtfs_rt_utils;
use crate::siri_model as model;
use crate::transit_realtime;
use actix_web::{error, Json, Query, Result, State};
use bytes::IntoBuf;
use log::{info, warn};
use navitia_model::collection::Idx;
use navitia_model::objects::StopPoint;
use prost::Message;
use serde;

fn current_datetime() -> model::DateTime {
    //TODO better datetime handling (if the server is not in the dataset's timezone it might lead to problems)
    model::DateTime(chrono::Local::now().naive_local())
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
enum DataFreshness {
    RealTime,
    Scheduled,
}

impl Default for DataFreshness {
    fn default() -> Self {
        DataFreshness::RealTime
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Params {
    _requestor_ref: Option<String>,
    monitoring_ref: String,
    _line_ref: Option<String>,
    _destination_ref: Option<String>,
    #[serde(default = "current_datetime")]
    start_time: model::DateTime,
    #[serde(skip)] //TODO
    _preview_interval: Option<chrono::Duration>,
    #[serde(default = "DataFreshness::default")]
    data_freshness: DataFreshness,
}

fn create_monitored_stop_visit(data: &Data, connection: &Connection) -> model::MonitoredStopVisit {
    let stop = &data.ntm.stop_points[connection.stop_point_idx];
    let vj = &data.ntm.vehicle_journeys[connection.dated_vj.vj_idx];
    let update_time = connection.realtime_info.as_ref().map(|c| c.update_time);
    let call = model::MonitoredCall {
        order: connection.sequence as u16,
        stop_point_name: stop.name.clone(),
        vehicle_at_stop: None,
        destination_display: None,
        aimed_arrival_time: Some(model::DateTime(connection.arr_time)),
        aimed_departure_time: Some(model::DateTime(connection.dep_time)),
        expected_arrival_time: connection
            .realtime_info
            .as_ref()
            .and_then(|c| c.arr_time)
            .map(model::DateTime),
        expected_departure_time: connection
            .realtime_info
            .as_ref()
            .and_then(|c| c.dep_time)
            .map(model::DateTime),
    };
    model::MonitoredStopVisit {
        monitoring_ref: stop.id.clone(),
        monitoring_vehicle_journey: model::MonitoredVehicleJourney {
            line_ref: vj.route_id.clone(),
            operator_ref: None,
            journey_pattern_ref: None,
            monitored_call: Some(call),
        },
        recorded_at_time: update_time,
        item_identifier: "".into(),
    }
}

fn create_stop_monitoring(
    stop_idx: Idx<StopPoint>,
    data: &Data,
    request: &Params,
) -> Vec<model::StopMonitoringDelivery> {
    let stop_visit = data
        .timetable
        .connections
        .iter()
        .skip_while(|c| c.dep_time < request.start_time.0)
        .filter(|c| c.stop_point_idx == stop_idx)
        // .filter() // filter on lines
        .map(|c| create_monitored_stop_visit(data, c))
        .take(2) //TODO make it a param
        .collect();

    vec![model::StopMonitoringDelivery {
        monitored_stop_visits: stop_visit,
    }]
}

// modify the generated timetable with a given GTFS-RT
// Since the connection are sorted by scheduled departure time we don't need to reorder the connections, we can update them in place
// For each trip update, we only have to find the corresponding connection and update it.
fn apply_rt_update(data: &mut Data, gtfs_rt: &transit_realtime::FeedMessage) -> Result<()> {
    let parsed_trip_update = gtfs_rt_utils::get_model_update(&data.ntm, gtfs_rt)?;
    let mut nb_changes = 0;

    for connection in &mut data.timetable.connections {
        let trip_update = parsed_trip_update.trips.get(&connection.dated_vj);
        if let Some(trip_update) = trip_update {
            let stop_time_update = trip_update
                .stop_time_update_by_sequence
                .get(&connection.sequence);
            if let Some(stop_time_update) = stop_time_update {
                // integrity check
                if stop_time_update.stop_point_idx != connection.stop_point_idx {
                    warn!("for trip {}, invalid stop connection, the stop n.{} '{}' does not correspond to the gtfsrt stop '{}'",
                    &data.ntm.vehicle_journeys[connection.dated_vj.vj_idx].id,
                    &connection.sequence,
                    &data.ntm.stop_points[connection.stop_point_idx].id,
                    &data.ntm.stop_points[stop_time_update.stop_point_idx].id,
                    );
                    continue;
                }
                connection.realtime_info = Some(RealTimeConnection {
                    dep_time: stop_time_update.updated_departure,
                    arr_time: stop_time_update.updated_arrival,
                    schedule_relationship: ScheduleRelationship::Scheduled,
                    update_time: trip_update.update_dt,
                });
                nb_changes += 1;
            } else {
                continue;
            }
        } else {
            // no trip update for this vehicle journey, we can skip
            continue;
        }
    }

    info!(
        "{} connections have been updated with trip updates",
        nb_changes
    );

    Ok(())
}

fn apply_latest_rt_update(context: &Context) -> actix_web::Result<()> {
    let gtfs_rt = context.gtfs_rt.lock().unwrap();

    let mut data = context.data.lock().unwrap();

    info!("applying realtime data on the scheduled data");
    let feed_message = gtfs_rt
        .as_ref()
        .map(|d| {
            transit_realtime::FeedMessage::decode((&d.data).into_buf()).map_err(|e| {
                error::ErrorInternalServerError(format!(
                    "impossible to decode protobuf message: {}",
                    e
                ))
            })
        })
        .ok_or_else(|| error::ErrorInternalServerError("impossible to access stored data"))??;

    apply_rt_update(&mut data, &feed_message)
}

fn realtime_update(context: &Context) -> actix_web::Result<()> {
    gtfs_rt_utils::update_gtfs_rt(context).map_err(error::ErrorInternalServerError)?;

    apply_latest_rt_update(context)
}

pub fn stop_monitoring(
    (state, query): (State<Context>, Query<Params>),
) -> Result<Json<model::SiriResponse>> {
    let request = query.into_inner();
    if request.data_freshness == DataFreshness::RealTime {
        realtime_update(&*state)?;
    }
    let arc_data = state.data.clone();
    let data = arc_data.lock().unwrap();

    let stop_idx = data
        .ntm
        .stop_points
        .get_idx(&request.monitoring_ref)
        .ok_or_else(|| {
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
                stop_monitoring_delivery: create_stop_monitoring(stop_idx, &data, &request),
            }),
            ..Default::default()
        },
    }))
}
