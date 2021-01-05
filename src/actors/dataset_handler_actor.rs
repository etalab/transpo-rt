use crate::datasets::{Dataset, RealTimeDataset};
use log::info;
use std::sync::Arc;

/// Actor whose role is to:
///  * give a pointer to a Dataset (on the GetDataset Message)
///  * update the pointer to a new Dataset (on the UpdateBaseSchedule Message)
pub struct DatasetActor {
    pub gtfs: Arc<Result<Dataset, anyhow::Error>>,
    pub realtime: Arc<RealTimeDataset>,
}

impl actix::Actor for DatasetActor {
    type Context = actix::Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Starting the context actor");
    }
}

#[derive(actix::Message)]
#[rtype(result = "Arc<Result<Dataset, anyhow::Error>>")]
pub struct GetDataset;

impl actix::Handler<GetDataset> for DatasetActor {
    type Result = Arc<Result<Dataset, anyhow::Error>>;

    fn handle(&mut self, _params: GetDataset, _ctx: &mut actix::Context<Self>) -> Self::Result {
        // we return a new Arc on the dataset
        self.gtfs.clone()
    }
}

#[derive(actix::Message)]
#[rtype(result = "Arc<RealTimeDataset>")]
pub struct GetRealtimeDataset;

impl actix::Handler<GetRealtimeDataset> for DatasetActor {
    type Result = Arc<RealTimeDataset>;

    fn handle(
        &mut self,
        _params: GetRealtimeDataset,
        _ctx: &mut actix::Context<Self>,
    ) -> Self::Result {
        self.realtime.clone()
    }
}
