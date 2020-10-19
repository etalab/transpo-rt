use crate::actors::{DatasetActor, GetDataset};
use crate::routes::{Link, Links};
use actix::Addr;
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
    dataset_actor: web::Data<Addr<DatasetActor>>,
) -> web::Json<Status> {
    let dataset = dataset_actor.send(GetDataset).await.unwrap(); //TODO remove this unwrap
    let url_for = |link: &str| Link {
        href: req
            .url_for(link, &[&dataset.feed_construction_info.dataset_info.id])
            .map(|u| u.into_string())
            .unwrap(),
        ..Default::default()
    };
    web::Json(Status {
        dataset: (&dataset.feed_construction_info.dataset_info).into(),
        loaded_at: dataset.loaded_at,
        links: btreemap! {
            "gtfs-rt" => url_for("gtfs-rt"),
            "gtfs-rt.json" => url_for("gtfs-rt.json"),
            "stop-monitoring" => url_for("stop-monitoring"),
            "stoppoints-discovery" => url_for("stoppoints-discovery"),
            "general-message" => url_for("general-message"),
            "siri-lite" => url_for("siri-lite"),
        }
        .into(),
    })
}
