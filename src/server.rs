use crate::context::{Context, Data, Period};
use crate::gtfs_rt::{gtfs_rt, gtfs_rt_json};
use crate::stop_monitoring::stop_monitoring_query;
use crate::stoppoints_discovery::sp_discovery;
use actix::{Actor, Addr};
use actix_web::{middleware, App};
use std::sync::{Arc, Mutex};

pub fn make_context(gtfs: &str, url: &str, generation_period: &Period) -> Context {
    let gtfs_rt_data = Arc::new(Mutex::new(None));
    let nav_data = if gtfs.starts_with("http") {
        navitia_model::gtfs::read_from_url(gtfs, None::<&str>, None).unwrap()
    } else {
        navitia_model::gtfs::read_from_zip(gtfs, None::<&str>, None).unwrap()
    };

    let data = Data::new(nav_data, &generation_period);
    let data = Arc::new(Mutex::new(data));
    Context {
        gtfs_rt: gtfs_rt_data.clone(),
        data: data.clone(),
        gtfs_rt_provider_url: url.to_owned(),
    }
}

pub fn create_server(context: Context) -> App<Addr<Context>> {
    let addr = context.start();
    App::with_state(addr)
        .middleware(middleware::Logger::default())
        .resource("/gtfs_rt", |r| r.f(gtfs_rt))
        .resource("/gtfs_rt.json", |r| r.f(gtfs_rt_json))
        .resource("/stoppoints_discovery.json", |r| r.with(sp_discovery))
        .resource("/stop_monitoring.json", |r| r.with(stop_monitoring_query))
}
