use actix_web::{middleware, App};
use crate::context::{lines_of_stop, Context};
use crate::gtfs_rt::{gtfs_rt, gtfs_rt_json};
use crate::stop_monitoring::stop_monitoring;
use crate::stoppoints_discovery::stoppoints_discovery;
use std::sync::{Arc, Mutex};

pub fn create_server(gtfs: &str, url: &str) -> App<Context> {
    let gtfs_rt_data = Arc::new(Mutex::new(None));
    let gtfs = if gtfs.starts_with("http") {
        gtfs_structures::Gtfs::from_url(gtfs).unwrap()
    } else {
        gtfs_structures::Gtfs::from_zip(gtfs).unwrap()
    };

    App::with_state(Context {
        gtfs_rt: gtfs_rt_data.clone(),
        lines_of_stops: gtfs
            .stops
            .values()
            .map(|stop| (stop.id.to_owned(), lines_of_stop(&gtfs, stop)))
            .collect(),
        gtfs,
        gtfs_rt_provider_url: url.to_owned(),
    }).middleware(middleware::Logger::default())
    .resource("/gtfs_rt", |r| r.f(gtfs_rt))
    .resource("/gtfs_rt.json", |r| r.f(gtfs_rt_json))
    .resource("/stoppoints_discovery.json", |r| {
        r.with(stoppoints_discovery)
    }).resource("/stop_monitoring.json", |r| r.with(stop_monitoring))
}
