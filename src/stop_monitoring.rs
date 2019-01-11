use crate::context::{Connection, Dataset, RealTimeConnection, RealTimeDataset, UpdatedTimetable};
use crate::dataset_handler_actor::{DatasetActor, GetRealtimeDataset};
use crate::siri_model as model;
use actix::Addr;
use actix_web::{error, AsyncResponder, Error, Json, Query, Result, State};
use futures::future::Future;
use navitia_model::collection::Idx;
use navitia_model::objects::StopPoint;

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

fn create_monitored_stop_visit(
    data: &Dataset,
    connection: &Connection,
    updated_connection: Option<&RealTimeConnection>,
) -> model::MonitoredStopVisit {
    let stop = &data.ntm.stop_points[connection.stop_point_idx];
    let vj = &data.ntm.vehicle_journeys[connection.dated_vj.vj_idx];
    let update_time = updated_connection.map(|c| c.update_time);
    let call = model::MonitoredCall {
        order: connection.sequence as u16,
        stop_point_name: stop.name.clone(),
        vehicle_at_stop: None,
        destination_display: None,
        aimed_arrival_time: Some(model::DateTime(connection.arr_time)),
        aimed_departure_time: Some(model::DateTime(connection.dep_time)),
        expected_arrival_time: updated_connection
            .and_then(|c| c.arr_time)
            .map(model::DateTime),
        expected_departure_time: updated_connection
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
    data: &Dataset,
    updated_timetable: &UpdatedTimetable,
    request: &Params,
) -> Vec<model::StopMonitoringDelivery> {
    let stop_visit = data
        .timetable
        .connections
        .iter()
        .enumerate()
        .skip_while(|(_, c)| c.dep_time < request.start_time.0)
        .filter(|(_, c)| c.stop_point_idx == stop_idx)
        // .filter() // filter on lines
        .map(|(idx, c)| {
            create_monitored_stop_visit(
                data,
                c,
                match request.data_freshness {
                    DataFreshness::RealTime => updated_timetable.realtime_connections.get(&idx),
                    DataFreshness::Scheduled => None,
                },
            )
        })
        .take(2) //TODO make it a param
        .collect();

    vec![model::StopMonitoringDelivery {
        monitored_stop_visits: stop_visit,
    }]
}

fn stop_monitoring(request: &Params, rt_data: &RealTimeDataset) -> Result<model::SiriResponse> {
    let data = &rt_data.base_schedule_dataset;
    let updated_timetable = &rt_data.updated_timetable;

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

    Ok(model::SiriResponse {
        siri: model::Siri {
            service_delivery: Some(model::ServiceDelivery {
                response_time_stamp: chrono::Local::now().to_rfc3339(),
                producer_ref: "".into(),
                address: "".into(),
                response_message_identifier: "".into(),
                request_message_ref: "".into(),
                stop_monitoring_delivery: create_stop_monitoring(
                    stop_idx,
                    data,
                    updated_timetable,
                    &request,
                ),
            }),
            ..Default::default()
        },
    })
}

pub fn stop_monitoring_query(
    (actor_addr, query): (State<Addr<DatasetActor>>, Query<Params>),
) -> Box<Future<Item = Json<model::SiriResponse>, Error = Error>> {
    actor_addr
        .send(GetRealtimeDataset)
        .map_err(Error::from)
        .and_then(|dataset| {
            dataset
                .and_then(|d| stop_monitoring(&query.into_inner(), &*d))
                .map(Json)
        })
        .responder()
}
