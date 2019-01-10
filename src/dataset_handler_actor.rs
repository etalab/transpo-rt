use crate::context::Dataset;
use log::info;
use std::sync::Arc;

/// Actor whose role is to:
///  * give a pointer to a Dataset (on the GetDataset Message)
///  * update the pointer to a new Dataset (on the UpdateBaseSchedule Message)
pub struct DatasetActor {
    pub gtfs: Arc<Dataset>,
    // pub real_time: Arc<RealTimeInfo>, // realtimeinfo = gtfs-rt + timetable ?
}

impl actix::Actor for DatasetActor {
    type Context = actix::Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Starting the context actor");
    }
}

pub struct GetDataset;

impl actix::Message for GetDataset {
    type Result = Result<Arc<Dataset>, actix_web::Error>;
}

impl actix::Handler<GetDataset> for DatasetActor {
    type Result = Result<Arc<Dataset>, actix_web::Error>;

    fn handle(&mut self, _params: GetDataset, _ctx: &mut actix::Context<Self>) -> Self::Result {
        // we return a new Arc on the dataset
        Ok(self.gtfs.clone())
    }
}
