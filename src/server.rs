use crate::context::{Context, Data, FeedConstructionInfo, Period};
use crate::gtfs_rt::{gtfs_rt, gtfs_rt_json};
use crate::stop_monitoring::stop_monitoring_query;
use crate::stoppoints_discovery::sp_discovery;
use actix::Addr;
use actix_web::{middleware, App};
use std::sync::{Arc, Mutex};

pub fn make_context(gtfs: &str, url: &str, generation_period: &Period) -> Context {
    let gtfs_rt_data = Arc::new(Mutex::new(None));

    let data = Data::from_path(gtfs, generation_period);
    let data = Arc::new(Mutex::new(data));
    Context {
        gtfs_rt: gtfs_rt_data.clone(),
        data: data.clone(),
        gtfs_rt_provider_url: url.to_owned(),
        feed_construction_info: FeedConstructionInfo {
            feed_path: gtfs.to_owned(),
            generation_period: generation_period.clone(),
        },
    }
}

pub fn create_server(addr: Addr<Context>) -> App<Addr<Context>> {
    App::with_state(addr)
        .middleware(middleware::Logger::default())
        .resource("/gtfs_rt", |r| r.f(gtfs_rt))
        .resource("/gtfs_rt.json", |r| r.f(gtfs_rt_json))
        .resource("/stoppoints_discovery.json", |r| r.with(sp_discovery))
        .resource("/stop_monitoring.json", |r| r.with(stop_monitoring_query))
}
