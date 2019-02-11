use crate::datasets::{DatasetInfo, Datasets};
use actix_web::{HttpRequest, Json};

/// Api to list all the hosted datasets
pub fn list_datasets(req: &HttpRequest<Datasets>) -> Json<Vec<DatasetInfo>> {
    Json(req.state().datasets.clone())
}
