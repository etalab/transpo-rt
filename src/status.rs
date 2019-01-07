use crate::context::Context;
use actix::{Addr, Handler};
use actix_web::{AsyncResponder, Error, HttpRequest, Json, Result};
use futures::future::Future;

struct Params;

#[derive(Serialize, Debug)]
pub struct Status {
    feed: String,
    loaded_at: chrono::DateTime<chrono::Utc>,
}

impl actix::Message for Params {
    type Result = Result<Status>;
}
impl Handler<Params> for Context {
    type Result = Result<Status>;

    fn handle(&mut self, _params: Params, _ctx: &mut actix::Context<Self>) -> Self::Result {
        Ok(Status {
            feed: self.feed_construction_info.feed_path.clone(),
            loaded_at: self.data.lock().unwrap().loaded_at,
        })
    }
}

pub fn status_query(
    req: &HttpRequest<Addr<Context>>,
) -> Box<Future<Item = Json<Status>, Error = Error>> {
    req.state()
        .send(Params {})
        .map_err(Error::from)
        .and_then(|result| result.map(Json))
        .responder()
}
