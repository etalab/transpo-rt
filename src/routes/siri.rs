use crate::extractors::DatasetWrapper;
use crate::routes::{Link, Links};
use actix_web::{web, HttpRequest};
use maplit::btreemap;

/// Api to tell what is available in siri-lite
pub async fn siri_endpoint(
    req: HttpRequest,
    dataset_wrapper: DatasetWrapper,
) -> actix_web::Result<web::Json<Links>> {
    let dataset = dataset_wrapper.get_dataset()?;
    let dataset_id = &dataset.feed_construction_info.dataset_info.id;
    Ok(web::Json(
        btreemap! {
            "stop-monitoring" => Link::from_scoped_url(&req, "stop_monitoring_query", dataset_id),
            "stoppoints-discovery" => Link::from_scoped_url(&req, "stoppoints_discovery_query", dataset_id),
            "general-message" => Link::from_scoped_url(&req, "general_message_query", dataset_id),
        }
        .into(),
    ))
}
