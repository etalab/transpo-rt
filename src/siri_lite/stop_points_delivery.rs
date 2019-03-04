use navitia_model::collection::Idx;
use navitia_model::objects::StopPoint;
use navitia_model::Model;

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

impl AnnotatedStopPoint {
    pub fn from(stop_point_idx: Idx<StopPoint>, model: &Model) -> Self {
        let lines = model
            .get_corresponding_from_idx(stop_point_idx)
            .into_iter()
            .map(|route_id| Line {
                line_ref: model.routes[route_id].id.clone(),
            })
            .collect();

        let sp = &model.stop_points[stop_point_idx];

        Self {
            stop_point_ref: sp.id.clone(),
            stop_name: sp.name.clone(),
            lines,
            location: Location {
                longitude: sp.coord.lon,
                latitude: sp.coord.lat,
            },
        }
    }
}
