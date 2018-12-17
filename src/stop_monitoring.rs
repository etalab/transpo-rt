use crate::context::{Connection, Context, Data};
use crate::siri_model as model;
use actix_web::{error, Json, Query, Result, State};
use gtfs_structures;
use log::info;
use navitia_model::collection::Idx;
use navitia_model::objects::StopPoint;
use serde;

fn current_datetime() -> model::DateTime {
    //TODO better datetime handling (if the server is not in the dataset's timezone it might lead to problems)
    model::DateTime(chrono::Local::now().naive_local())
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
}

fn create_monitored_stop_visit(data: &Data, connection: &Connection) -> model::MonitoredStopVisit {
    let stop = &data.raw.stop_points[connection.stop_point_idx];
    let vj = &data.raw.vehicle_journeys[connection.vj_idx];
    let call = model::MonitoredCall {
        order: connection.sequence as u16,
        stop_point_name: stop.name.clone(),
        vehicle_at_stop: None,
        destination_display: None,
        aimed_arrival_time: Some(model::DateTime(connection.arr_time.clone())),
        aimed_departure_time: Some(model::DateTime(connection.dep_time.clone())),
        expected_arrival_time: None,
        expected_departure_time: None,
    };
    model::MonitoredStopVisit {
        monitoring_ref: stop.id.clone(),
        monitoring_vehicle_journey: model::MonitoredVehicleJourney {
            line_ref: vj.route_id.clone(),
            operator_ref: None,
            journey_pattern_ref: None,
            monitored_call: Some(call),
        },
        recorded_at_time: "".into(),
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

pub fn stop_monitoring(
    (state, query): (State<Context>, Query<Params>),
) -> Result<Json<model::SiriResponse>> {
    let arc_data = state.data.clone();

    let data = arc_data.lock().unwrap();
    let stops = &data.raw.stop_points;

    let request = query.into_inner();
    info!("receiving :{:?}", &request);

    //TODO handle stop_area ?
    let stop_idx = stops.get_idx(&request.monitoring_ref).ok_or_else(|| {
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
