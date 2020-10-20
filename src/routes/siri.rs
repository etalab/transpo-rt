use crate::actors::{DatasetActor, GetDataset};
use crate::routes::{Link, Links};
use actix::Addr;
use actix_web::{get, web, HttpRequest};
use maplit::btreemap;

/// Api to tell what is available in siri-lite
#[get("/siri/2.0")]
pub async fn siri_endpoint(
    req: HttpRequest,
    dataset_actor: web::Data<Addr<DatasetActor>>,
) -> actix_web::Result<web::Json<Links>> {
    let dataset = dataset_actor.send(GetDataset).await.map_err(|e| {
        log::error!("error while querying actor for data: {:?}", e);
        actix_web::error::ErrorInternalServerError("impossible to get data".to_string())
    })?;
    let url_for = |link: &str| Link {
        href: req
            .url_for(link, &[&dataset.feed_construction_info.dataset_info.id])
            .map(|u| u.into_string())
            .unwrap_or_else(|_| panic!("impossible to find route {} to make a link", link)),
        ..Default::default()
    };
    Ok(web::Json(
        btreemap! {
            "stop-monitoring" => url_for("stop_monitoring_query"),
            "stoppoints-discovery" => url_for("stoppoints_discovery_query"),
            "general-message" => url_for("general_message_query"),
        }
        .into(),
    ))
}
