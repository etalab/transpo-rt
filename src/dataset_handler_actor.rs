use crate::context::{Dataset, RealTimeDataset};
use log::info;
use std::sync::Arc;

/// Actor whose role is to:
///  * give a pointer to a Dataset (on the GetDataset Message)
///  * update the pointer to a new Dataset (on the UpdateBaseSchedule Message)
pub struct DatasetActor {
    pub gtfs: Arc<Dataset>,
    pub realtime: Arc<RealTimeDataset>,
}

impl actix::Actor for DatasetActor {
    type Context = actix::Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Starting the context actor");
    }
}

pub struct GetDataset;

impl actix::Message for GetDataset {
    // TODO use Arc<Dataset> instead of Result<Arc<Dataset>>
    // cf https://github.com/actix/actix/pull/199
    type Result = Result<Arc<Dataset>, actix_web::Error>;
}

impl actix::Handler<GetDataset> for DatasetActor {
    // type Result = Arc<Dataset>;
    type Result = Result<Arc<Dataset>, actix_web::Error>;

    fn handle(&mut self, _params: GetDataset, _ctx: &mut actix::Context<Self>) -> Self::Result {
        // we return a new Arc on the dataset
        Ok(self.gtfs.clone())
    }
}

pub struct GetRealtimeDataset;

impl actix::Message for GetRealtimeDataset {
    // TODO use Arc<Dataset> instead of Result<Arc<Dataset>>
    // cf https://github.com/actix/actix/pull/199
    type Result = Result<Arc<RealTimeDataset>, actix_web::Error>;
}

impl actix::Handler<GetRealtimeDataset> for DatasetActor {
    type Result = Result<Arc<RealTimeDataset>, actix_web::Error>;

    fn handle(
        &mut self,
        _params: GetRealtimeDataset,
        _ctx: &mut actix::Context<Self>,
    ) -> Self::Result {
        Ok(self.realtime.clone())
    }
}
