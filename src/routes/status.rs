use crate::extractors::DatasetWrapper;
use crate::routes::{Link, Links};
use actix_web::{web, HttpRequest};
use maplit::btreemap;
use openapi_schema::OpenapiSchema;

#[derive(Serialize, Debug, OpenapiSchema)]
pub struct Status {
    #[serde(flatten)]
    dataset: super::ExposedDataset,
    loaded_at: chrono::DateTime<chrono::Utc>,
    #[serde(flatten)]
    pub links: Links,
}

pub async fn status_query(
    req: HttpRequest,
    dataset_wrapper: DatasetWrapper,
) -> actix_web::Result<web::Json<Status>> {
    let dataset = dataset_wrapper.get_dataset()?;

    let dataset_id = &dataset.feed_construction_info.dataset_info.id;

    Ok(web::Json(Status {
        dataset: (&dataset.feed_construction_info.dataset_info).into(),
        loaded_at: dataset.loaded_at,
        links: btreemap! {
            "gtfs-rt" => Link::from_scoped_url(&req, "gtfs_rt_protobuf", &dataset_id),
            "gtfs-rt.json" => Link::from_scoped_url(&req, "gtfs_rt_json", &dataset_id),
            "stop-monitoring" => Link::from_scoped_url(&req, "stop_monitoring_query", &dataset_id),
            "stoppoints-discovery" => Link::from_scoped_url(&req, "stoppoints_discovery_query", &dataset_id),
            "general-message" => Link::from_scoped_url(&req, "general_message_query", &dataset_id),
            "siri-lite" => Link::from_scoped_url(&req, "siri_endpoint", &dataset_id),
        }
        .into(),
    }))
}
