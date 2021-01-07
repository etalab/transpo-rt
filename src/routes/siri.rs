use crate::actors::{DatasetActor, GetDataset};
use crate::routes::{Link, Links};
use actix::Addr;
use actix_web::{web, HttpRequest};
use maplit::btreemap;

/// Api to tell what is available in siri-lite
pub async fn siri_endpoint(
    req: HttpRequest,
    dataset_actor: web::Data<Addr<DatasetActor>>,
) -> actix_web::Result<web::Json<Links>> {
    let dataset = dataset_actor
        .send(GetDataset)
        .await
        .map_err(|e| {
            log::error!("error while querying actor for data: {:?}", e);
            actix_web::error::ErrorInternalServerError("impossible to get data".to_string())
        })?
        .map_err(|e| {
            // TODO : log the error ?
            actix_web::error::ErrorInternalServerError(
                "theoretical dataset temporarily unavailable".to_string(),
            )
        })?;

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
