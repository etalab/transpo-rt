use crate::context::Data;

// TODO store a real date, and only change serialization
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DateTime(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorCondition {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Line {
    pub line_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AnnotatedStopPoint {
    pub stop_point_ref: String,
    pub stop_name: String,
    pub lines: Vec<Line>,
    pub location: Location,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StopPointsDelivery {
    pub version: String,
    pub response_time_stamp: String,
    pub status: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_condition: Option<ErrorCondition>,
    pub annotated_stop_point: Vec<AnnotatedStopPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MonitoredCall {
    pub order: u16,
    pub stop_point_name: String,
    /// true if the vehicle is at the stop
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vehicle_at_stop: Option<bool>,
    /// headsign of the vehicle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_display: Option<String>,
    /// Scheduled arrival time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aimed_arrival_time: Option<DateTime>,
    /// Scheduled departure time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aimed_departure_time: Option<DateTime>,
    /// Estimated arrival time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_arrival_time: Option<DateTime>,
    /// Estimated departure time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_departure_time: Option<DateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MonitoredVehicleJourney {
    pub line_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journey_pattern_ref: Option<String>,
    pub monitored_call: Option<MonitoredCall>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MonitoredStopVisit {
    pub monitoring_ref: String,
    pub monitoring_vehicle_journey: MonitoredVehicleJourney,
    pub recorded_at_time: String,
    /// Id of the couple Stop / VehicleJourney
    pub item_identifier: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StopMonitoringDelivery {
    pub monitored_stop_visits: Vec<MonitoredStopVisit>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceDelivery {
    pub response_time_stamp: String,
    pub producer_ref: String,
    pub address: String,
    pub response_message_identifier: String,
    pub request_message_ref: String,
    pub stop_monitoring_delivery: Vec<StopMonitoringDelivery>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Siri {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_points_delivery: Option<StopPointsDelivery>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_delivery: Option<ServiceDelivery>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SiriResponse {
    pub siri: Siri,
}

impl AnnotatedStopPoint {
    pub fn from(stop: &gtfs_structures::Stop, data: &Data) -> Self {
        let lines = data
            .lines_of_stops
            .get(&stop.id)
            .unwrap_or(&std::collections::HashSet::new())
            .iter()
            .map(|route_id| Line {
                line_ref: route_id.to_owned(),
            }).collect();

        Self {
            stop_point_ref: stop.id.to_owned(),
            stop_name: stop.name.to_owned(),
            lines,
            location: Location {
                longitude: stop.longitude,
                latitude: stop.latitude,
            },
        }
    }
}
