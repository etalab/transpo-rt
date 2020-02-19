use crate::actors::{DatasetActor, GetDataset};
use crate::routes::{Link, Links};
use actix::Addr;
use actix_web::{AsyncResponder, Error, HttpRequest, Json};
use futures::future::Future;
use maplit::btreemap;

/// Api to tell what is available in siri-lite
pub fn siri_endpoint(
    req: HttpRequest<Addr<DatasetActor>>,
) -> Box<dyn Future<Item = Json<Links>, Error = Error>> {
    req.state()
        .send(GetDataset)
        .map_err(Error::from)
        .and_then(|dataset| {
            dataset.map(move |d| {
                let url_for = |link: &str| Link {
                    href: req
                        .url_for(link, &[&d.feed_construction_info.dataset_info.id])
                        .map(|u| u.into_string())
                        .unwrap(),
                    ..Default::default()
                };
                Json(
                    btreemap! {
                        "stop-monitoring" => url_for("stop-monitoring"),
                        "stoppoints-discovery" => url_for("stoppoints-discovery"),
                        "general-message" => url_for("general-message"),
                    }
                    .into(),
                )
            })
        })
        .responder()
}
