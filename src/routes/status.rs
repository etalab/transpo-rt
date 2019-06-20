use crate::actors::{DatasetActor, GetDataset};
use crate::routes::{Link, Links};
use actix::Addr;
use actix_web::{AsyncResponder, Error, HttpRequest, Json};
use futures::future::Future;
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

pub fn status_query(
    req: &HttpRequest<Addr<DatasetActor>>,
) -> Box<Future<Item = Json<Status>, Error = Error>> {
    let q = req.clone();
    req.state()
        .send(GetDataset)
        .map_err(Error::from)
        .and_then(|dataset| {
            dataset.map(move |d| {
                let url_for = |link: &str| Link {
                    href: q
                        .url_for(link, &[&d.feed_construction_info.dataset_info.id])
                        .map(|u| u.into_string())
                        .unwrap(),
                    ..Default::default()
                };
                Json(Status {
                    dataset: (&d.feed_construction_info.dataset_info).into(),
                    loaded_at: d.loaded_at,
                    links: btreemap! {
                        "gtfs-rt" => url_for("gtfs-rt"),
                        "gtfs-rt.json" => url_for("gtfs-rt.json"),
                        "stop-monitoring" => url_for("stop-monitoring"),
                        "stoppoints-discovery" => url_for("stoppoints-discovery"),
                        "general-message" => url_for("general-message"),
                    }
                    .into(),
                })
            })
        })
        .responder()
}
