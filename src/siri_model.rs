use crate::context::Context;

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
    pub error_condition: Option<ErrorCondition>,
    pub annotated_stop_point: Vec<AnnotatedStopPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Siri {
    pub stop_points_delivery: StopPointsDelivery,
}

impl AnnotatedStopPoint {
    pub fn from(stop: &gtfs_structures::Stop, context: &Context) -> Self {
        let lines = context
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
