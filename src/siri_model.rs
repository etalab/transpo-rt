use crate::context::Stop;
use navitia_model::Model;

#[derive(Debug)]
pub struct DateTime(pub chrono::DateTime<chrono::Local>);

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
    pub fn from(stop: &Stop, model: &Model) -> Self {
        let lines = match stop {
            Stop::StopPoint(sp) => model.get_corresponding_from_idx(*sp),
            Stop::StopArea(sa) => model.get_corresponding_from_idx(*sa),
        }
        .into_iter()
        .map(|route_id| Line {
            line_ref: model.routes[route_id].id.clone(),
        })
        .collect();

        let (id, name, lon, lat) = match stop {
            Stop::StopPoint(idx) => {
                let sp = &model.stop_points[*idx];
                (sp.id.clone(), sp.name.clone(), sp.coord.lon, sp.coord.lat)
            }
            Stop::StopArea(idx) => {
                let sp = &model.stop_areas[*idx];
                (sp.id.clone(), sp.name.clone(), sp.coord.lon, sp.coord.lat)
            }
        };

        Self {
            stop_point_ref: id,
            stop_name: name,
            lines,
            location: Location {
                longitude: lon,
                latitude: lat,
            },
        }
    }
}

impl serde::Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_rfc3339())
    }
}

impl<'de> ::serde::Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        use chrono::offset::TimeZone;
        use serde::de::Error;

        let date_str = String::deserialize(deserializer)?;
        let dt = chrono::DateTime::parse_from_rfc3339(&date_str).map_err(Error::custom)?;
        let offset = chrono::Local::from_offset(&dt.offset());
        let dt = dt.with_timezone(&offset);

        Ok(DateTime(dt))
    }
}
