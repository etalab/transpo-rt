use crate::actors::{DatasetActor, GetRealtimeDataset};
use crate::datasets::{Connection, Dataset, RealTimeConnection, RealTimeDataset, UpdatedTimetable};
use crate::siri_model as model;
use actix::Addr;
use actix_web::{error, AsyncResponder, Error, Json, Query, Result, State};
use futures::future::Future;
use navitia_model::collection::Idx;
use navitia_model::objects::StopPoint;

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
    /// Id of the stop_point on which we want the next departures
    monitoring_ref: String,
    line_ref: Option<String>,
    _destination_ref: Option<String>,
    /// start_time is the datetime from which we want the next departures
    /// The default is the current time of the query
    start_time: Option<model::DateTime>,
    #[serde(skip)] //TODO
    _preview_interval: Option<chrono::Duration>,
    /// the data_freshness is used to control whether we want realtime data or only base schedule data
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
    let route = &data.ntm.routes.get(&vj.route_id);
    // we consider that the siri's operator in transmodel's company
    let operator_ref = data
        .ntm
        .get_corresponding_from_idx(connection.dated_vj.vj_idx)
        .into_iter()
        .next()
        .map(|idx| data.ntm.companies[idx].id.clone());
    let line_ref = route
        .map(|r| r.line_id.clone())
        .unwrap_or_else(|| "".to_owned());
    let update_time = updated_connection
        .map(|c| c.update_time)
        // if we have no realtime data, we consider the update time to be the time of the base schedule loading
        // (it's not that great, but we don't have something better)
        .unwrap_or_else(|| data.loaded_at);
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
            line_ref,
            service_info: model::ServiceInfoGroup { operator_ref },
            journey_pattern_ref: None,
            monitored_call: Some(call),
        },
        recorded_at_time: update_time,
        item_identifier: format!("{}:{}", &stop.id, &vj.id),
    }
}

fn get_line_ref<'a>(cnx: &Connection, model: &'a navitia_model::Model) -> Option<&'a str> {
    let vj = &model.vehicle_journeys[cnx.dated_vj.vj_idx];
    model.routes.get(&vj.route_id).map(|r| r.line_id.as_str())
}

fn create_stop_monitoring(
    stop_idx: Idx<StopPoint>,
    data: &Dataset,
    updated_timetable: &UpdatedTimetable,
    request: &Params,
) -> Vec<model::StopMonitoringDelivery> {
    // if we want to datetime in the query, we get the current_time (in the timezone of the dataset)
    let requested_start_time = request.start_time.as_ref().map(|d| d.0).unwrap_or_else(|| {
        chrono::Utc::now()
            .with_timezone(&data.timezone)
            .naive_local()
    });
    let requested_line_ref = request.line_ref.as_ref().map(String::as_str);
    let stop_visit = data
        .timetable
        .connections
        .iter()
        .enumerate()
        .skip_while(|(_, c)| c.dep_time < requested_start_time)
        .filter(|(_, c)| c.stop_point_idx == stop_idx)
        // filter on requested lines
        .filter(|(_, c)| {
            requested_line_ref.is_none() || requested_line_ref == get_line_ref(&c, &data.ntm)
        })
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
        version: "2.0".to_owned(),
        response_time_stamp: chrono::Local::now().to_rfc3339(),
        request_message_ref: None,
        status: true,
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
                producer_ref: None, // TODO take the id of the dataset ?
                address: None,
                response_message_identifier: None,
                request_message_ref: None, // TODO if a request ref is given in the query, return it
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
