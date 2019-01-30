use crate::actors::{DatasetActor, GetDataset};
use crate::context::{
    Dataset, GtfsRT, RealTimeConnection, RealTimeDataset, ScheduleRelationship, UpdatedTimetable,
};
use crate::gtfs_rt_utils;
use crate::transit_realtime;
use actix::fut::ActorFuture;
use actix::fut::WrapFuture;
use actix::prelude::ContextFutureSpawner;
use actix::AsyncContext;
use failure::format_err;
use failure::Error;
use futures::future::Future;
use log::info;
use prost::Message;
use std::io::Read;
use std::sync::Arc;

/// Actor that once in a while reload the BaseSchedule data (GTFS)
/// and send them to the DatasetActor
pub struct RealTimeReloader {
    pub gtfs_rt_urls: Vec<String>,

    // Address of the DatasetActor to notify for the data reloading
    // NOte: for the moment it's a single Actor,
    // but if we have several instances of DatasetActor we could have a list of recipient here
    pub dataset_actor: actix::Addr<DatasetActor>,
}

// TODO: make this async
fn fetch_gtfs_rt(url: &str) -> Result<GtfsRT, Error> {
    info!("fetching a gtfs_rt");
    let gtfs_rt = reqwest::get(url)
        .and_then(|resp| resp.error_for_status())
        .map_err(|e| format_err!("Unable to fetch the gtfs RT {}", e))?
        .bytes()
        .collect::<Result<Vec<u8>, _>>()?;
    Ok(GtfsRT {
        data: gtfs_rt,
        datetime: chrono::Utc::now(),
    })
}

// modify the generated timetable with a given GTFS-RT
// Since the connection are sorted by scheduled departure time we don't need to reorder the connections, we can update them in place
// For each trip update, we only have to find the corresponding connection and update it.
fn apply_rt_update(
    data: &Dataset,
    gtfs_rt: &transit_realtime::FeedMessage,
) -> Result<UpdatedTimetable, Error> {
    let mut updated_timetable = UpdatedTimetable::default();

    let parsed_trip_update = gtfs_rt_utils::get_model_update(&data.ntm, gtfs_rt, data.timezone)?;
    let mut nb_changes = 0;

    for (idx, connection) in &mut data.timetable.connections.iter().enumerate() {
        let trip_update = parsed_trip_update.trips.get(&connection.dated_vj);
        if let Some(trip_update) = trip_update {
            let stop_time_update = trip_update
                .stop_time_update_by_sequence
                .get(&connection.sequence);
            if let Some(stop_time_update) = stop_time_update {
                // integrity check
                if stop_time_update.stop_point_idx != connection.stop_point_idx {
                    log::warn!("for trip {}, invalid stop connection, the stop n.{} '{}' does not correspond to the gtfsrt stop '{}'",
                    &data.ntm.vehicle_journeys[connection.dated_vj.vj_idx].id,
                    &connection.sequence,
                    &data.ntm.stop_points[connection.stop_point_idx].id,
                    &data.ntm.stop_points[stop_time_update.stop_point_idx].id,
                    );
                    continue;
                }
                updated_timetable.realtime_connections.insert(
                    idx,
                    RealTimeConnection {
                        dep_time: stop_time_update.updated_departure,
                        arr_time: stop_time_update.updated_arrival,
                        schedule_relationship: ScheduleRelationship::Scheduled,
                        update_time: trip_update.update_dt,
                    },
                );
                nb_changes += 1;
            } else {
                continue;
            }
        } else {
            // no trip update for this vehicle journey, we can skip
            continue;
        }
    }

    info!(
        "{} connections have been updated with trip updates",
        nb_changes
    );

    Ok(updated_timetable)
}

impl RealTimeReloader {
    fn update_realtime_data(&self, ctx: &mut actix::Context<Self>) {
        // we fetch the latest baseschedule data
        self.dataset_actor
            .send(GetDataset)
            .map_err(|e| format!("impossible to fetch baseschedule data: {}", e))
            .into_actor(self)
            .then(|res, act, _| {
                match res
                    .map_err(|e| format_err!("maibox error: {}", e))
                    .and_then(|dataset| act.apply_rt(dataset.unwrap()))
                {
                    Ok(()) => {
                        info!("realtime reloaded");
                    }
                    Err(e) => {
                        log::error!("unable to apply realtime update due to: {}", e);
                    }
                }
                // Note: this return value is not very useful as the `wait(ctx)` function below does not handle the return value
                actix::fut::ok(())
            })
            .wait(ctx);
    }

    fn apply_rt(&self, dataset: Arc<Dataset>) -> Result<(), Error> {
        //TODO: make this async
        if let Some(url) = self.gtfs_rt_urls.first() {
            let gtfs_rt = fetch_gtfs_rt(url)?;

            // we compute the new timetable
            let rt_dataset = self.make_rt_dataset(dataset, gtfs_rt)?;

            // we send those data as a BaseScheduleReloader message, for the DatasetActor to load those new data
            self.dataset_actor
                .do_send(UpdateRealtime(Arc::new(rt_dataset)));
            Ok(())
        } else {
            Err(format_err!("No url!"))
        }
    }

    fn make_rt_dataset(
        &self,
        dataset: Arc<Dataset>,
        gtfs_rt: GtfsRT,
    ) -> Result<RealTimeDataset, Error> {
        use bytes::IntoBuf;
        let feed_message = transit_realtime::FeedMessage::decode(
            gtfs_rt
                .data
                .clone() // we need to clone the gtfs_rt because we want to keep it for proxy use, and still to decode it
                .into_buf(),
        )
        .map_err(|e| format_err!("impossible to decode protobuf message: {}", e))?;

        let updated_timetable = apply_rt_update(&dataset, &feed_message)?;

        Ok(RealTimeDataset {
            base_schedule_dataset: dataset,
            gtfs_rt: Some(gtfs_rt),
            gtfs_rt_provider_urls: self.gtfs_rt_urls.clone(),
            updated_timetable,
        })
    }
}

impl actix::Actor for RealTimeReloader {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting the realtime updater actor");

        self.update_realtime_data(ctx);
        ctx.run_interval(std::time::Duration::from_secs(60), |act, ctx| {
            info!("reloading realtime data");
            act.update_realtime_data(ctx);
        });
    }
}

/// Message send to a DatasetActor to update its baseschedule data
struct UpdateRealtime(Arc<RealTimeDataset>);

impl actix::Message for UpdateRealtime {
    type Result = ();
}

impl actix::Handler<UpdateRealtime> for DatasetActor {
    type Result = ();

    fn handle(&mut self, params: UpdateRealtime, _ctx: &mut actix::Context<Self>) -> Self::Result {
        self.realtime = params.0;
    }
}
