use crate::datasets::Datasets;
use crate::routes::{ExposedDataset, Link, Links};
use actix_web::{HttpRequest, Json};
use maplit::btreemap;
use openapi_schema::OpenapiSchema;

#[derive(Serialize, Debug, OpenapiSchema)]
pub struct ApiEntryPoint {
    pub datasets: Vec<ExposedDataset>,
    #[serde(flatten)]
    pub links: Links,
}

fn raw_url(req: &HttpRequest<Datasets>, u: &str) -> String {
    // since there are several App in actix, we can't call url_for for datasets route
    // so we hardcode them
    let conn = req.connection_info();
    format!("{}://{}{}", conn.scheme(), conn.host(), u)
}

/// Api to list all the hosted datasets
pub fn api_entry_point(req: &HttpRequest<Datasets>) -> Json<ApiEntryPoint> {
    Json(ApiEntryPoint {
        datasets: req
            .state()
            .datasets
            .iter()
            .map(|d| {
                ExposedDataset::from(d)
                    .add_link("self", &raw_url(req, &format!("/{id}/", id = &d.id)))
            })
            .collect(),
        links: btreemap! {
            "documentation" => Link::from_url(req, "documentation", &[]),
            "dataset_detail" => Link {
                href: raw_url(req, "/{id}/"),
                templated: Some(true),
            }
        }
        .into(),
    })
}
