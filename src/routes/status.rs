use crate::actors::{DatasetActor, GetDataset};
use actix::Addr;
use actix_web::{AsyncResponder, Error, HttpRequest, Json};
use futures::future::Future;

#[derive(Serialize, Debug)]
pub struct Status {
    feed: String,
    loaded_at: chrono::DateTime<chrono::Utc>,
}

pub fn status_query(
    req: &HttpRequest<Addr<DatasetActor>>,
) -> Box<Future<Item = Json<Status>, Error = Error>> {
    req.state()
        .send(GetDataset)
        .map_err(Error::from)
        .and_then(|dataset| {
            dataset.map(|d| {
                Json(Status {
                    feed: d.feed_construction_info.feed_path.clone(),
                    loaded_at: d.loaded_at,
                })
            })
        })
        .responder()
}
