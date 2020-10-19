use crate::actors::{DatasetActor, GetDataset};
use crate::routes::{Link, Links};
use actix::Addr;
use actix_web::{ HttpRequest, web, get};
use maplit::btreemap;

/// Api to tell what is available in siri-lite
#[get("/siri/2.0")]
pub async fn siri_endpoint(
    req: HttpRequest,
    dataset_actor: web::Data<Addr<DatasetActor>>,
) -> actix_web::Result<web::Json<Links>> {
    let dataset = dataset_actor.send(GetDataset).await.map_err(|e| {
        log::error!("error while querying actor for data: {:?}", e);
        actix_web::error::ErrorInternalServerError(format!("impossible to get data",))
    })?;
    let url_for = |link: &str| Link {
        href: req
            .url_for(link, &[&dataset.feed_construction_info.dataset_info.id])
            .map(|u| u.into_string())
            .unwrap(),
        ..Default::default()
    };
    Ok(web::Json(
        btreemap! {
            "stop-monitoring" => url_for("stop-monitoring"),
            "stoppoints-discovery" => url_for("stoppoints-discovery"),
            "general-message" => url_for("general-message"),
        }
        .into(),
    ))
}
