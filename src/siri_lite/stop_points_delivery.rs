use transit_model::collection::Idx;
use transit_model::objects::StopPoint;
use transit_model::Model;

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
    #[serde(flatten)]
    pub common: crate::siri_lite::shared::CommonDelivery,
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
