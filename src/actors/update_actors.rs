use crate::actors::DatasetActor;
use crate::datasets::{Dataset, FeedConstructionInfo};
use actix::AsyncContext;
use slog::info;
use std::sync::Arc;

/// Actor that once in a while reload the BaseSchedule data (GTFS)
/// and send them to the DatasetActor
pub struct BaseScheduleReloader {
    pub feed_construction_info: FeedConstructionInfo,

    // Address of the DatasetActor to notify for the data reloading
    // NOte: for the moment it's a single Actor,
    // but if we have several instances of DatasetActor we could have a list of recipient here
    pub dataset_actor: actix::Addr<DatasetActor>,
    pub log: slog::Logger,
}

impl BaseScheduleReloader {
    fn update_data(&self, ctx: &mut actix::Context<Self>) {
        slog_scope::scope(&self.log, || {
            let new_dataset = Dataset::try_from_dataset_info(
                self.feed_construction_info.dataset_info.clone(),
                &crate::datasets::Period {
                    begin: chrono::Local::today().naive_local(),
                    horizon: self.feed_construction_info.generation_period.horizon,
                },
            );

            if let Err(e) = new_dataset {
                log::warn!("impossible to update dataset because of: {}", e);
                log::warn!("rescheduling data loading in 5 mn");

                // trace error in sentry
                sentry::Hub::current().configure_scope(|scope| {
                    scope.set_tag("dataset", &self.feed_construction_info.dataset_info.id);
                });
                sentry::integrations::anyhow::capture_anyhow(&e);

                ctx.run_later(std::time::Duration::from_secs(5 * 60), |act, ctx| {
                    act.update_data(ctx)
                });
            }
            self.dataset_actor
                .do_send(UpdateBaseSchedule(Arc::new(new_dataset)));
        });
    }
}

impl actix::Actor for BaseScheduleReloader {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(self.log, "Starting the base schedule updater actor");
        ctx.run_interval(std::time::Duration::from_secs(60 * 60 * 24), |act, ctx| {
            info!(act.log, "reloading baseschedule data");
            act.update_data(ctx);
        });
    }
}

/// Message send to a DatasetActor to update its baseschedule data
struct UpdateBaseSchedule(Arc<Result<Dataset, anyhow::Error>>);

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
