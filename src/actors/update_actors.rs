use crate::actors::DatasetActor;
use crate::context::{Dataset, FeedConstructionInfo};
use actix::AsyncContext;
use log::info;
use std::sync::Arc;

/// Actor that once in a while reload the BaseSchedule data (GTFS)
/// and send them to the DatasetActor
pub struct BaseScheduleReloader {
    pub feed_construction_info: FeedConstructionInfo,

    // Address of the DatasetActor to notify for the data reloading
    // NOte: for the moment it's a single Actor,
    // but if we have several instances of DatasetActor we could have a list of recipient here
    pub dataset_actor: actix::Addr<DatasetActor>,
}

impl BaseScheduleReloader {
    fn update_data(&self) {
        let new_dataset = Dataset::from_path(
            &self.feed_construction_info.feed_path,
            &crate::context::Period {
                begin: chrono::Local::today().naive_local(),
                horizon: self.feed_construction_info.generation_period.horizon,
            },
        );

        // we send those data as a BaseScheduleReloader message, for the DatasetActor to load those new data
        self.dataset_actor
            .do_send(UpdateBaseSchedule(Arc::new(new_dataset)));
    }
}

impl actix::Actor for BaseScheduleReloader {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting the base schedule updater actor");
        ctx.run_interval(std::time::Duration::from_secs(60 * 60 * 24), |act, _ctx| {
            info!("reloading baseschedule data");
            act.update_data();
        });
    }
}

/// Message send to a DatasetActor to update its baseschedule data
struct UpdateBaseSchedule(Arc<Dataset>);

impl actix::Message for UpdateBaseSchedule {
    type Result = ();
}

impl actix::Handler<UpdateBaseSchedule> for DatasetActor {
    type Result = ();

    fn handle(
        &mut self,
        params: UpdateBaseSchedule,
        _ctx: &mut actix::Context<Self>,
    ) -> Self::Result {
        self.gtfs = params.0;
    }
}
