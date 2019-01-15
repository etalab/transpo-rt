use crate::actors::{BaseScheduleReloader, DatasetActor, RealTimeReloader};
use crate::context;
use crate::context::{Dataset, Period};
use crate::gtfs_rt::{gtfs_rt, gtfs_rt_json};
use crate::status::status_query;
use crate::stop_monitoring::stop_monitoring_query;
use crate::stoppoints_discovery::sp_discovery;
use actix::Actor;
use actix::Addr;
use actix_web::middleware::cors::Cors;
use actix_web::{middleware, App};
use std::sync::Arc;

#[derive(Deserialize, Debug)]
pub struct DatasetToLoad {
    pub name: String,
    pub id: String,
    pub gtfs: String,
    pub gtfs_rt: String,
}

impl DatasetToLoad {
    pub fn new_default(gtfs: &str, gtfs_rt: &str) -> Self {
        Self {
            id: "default".into(),
            name: "default".into(),
            gtfs: gtfs.to_owned(),
            gtfs_rt: gtfs_rt.to_owned(),
        }
    }
}

pub fn create_all_actors(
    dataset_info: &DatasetToLoad,
    generation_period: &Period,
) -> Addr<DatasetActor> {
    let dataset = Dataset::from_path(&dataset_info.gtfs, &generation_period);
    let arc_dataset = Arc::new(dataset);
    let rt_dataset = context::RealTimeDataset::new(arc_dataset.clone(), &dataset_info.gtfs_rt);
    let dataset_actors = DatasetActor {
        gtfs: arc_dataset.clone(),
        realtime: Arc::new(rt_dataset),
    };
    let dataset_actors_addr = dataset_actors.start();
    let base_schedule_reloader = BaseScheduleReloader {
        feed_construction_info: context::FeedConstructionInfo {
            feed_path: dataset_info.gtfs.clone(),
            generation_period: generation_period.clone(),
        },
        dataset_actor: dataset_actors_addr.clone(),
    };
    base_schedule_reloader.start();
    let realtime_reloader = RealTimeReloader {
        gtfs_rt_url: dataset_info.gtfs_rt.clone(),
        dataset_actor: dataset_actors_addr.clone(),
    };
    realtime_reloader.start();

    dataset_actors_addr
}

pub fn create_server(addr: Addr<DatasetActor>) -> App<Addr<DatasetActor>> {
    App::with_state(addr)
        .middleware(middleware::Logger::default())
        .middleware(Cors::build().allowed_methods(vec!["GET"]).finish())
        .resource("/status", |r| r.f(status_query))
        .resource("/gtfs_rt", |r| r.f(gtfs_rt))
        .resource("/gtfs_rt.json", |r| r.f(gtfs_rt_json))
        .resource("/stoppoints_discovery.json", |r| r.with(sp_discovery))
        .resource("/stop_monitoring.json", |r| r.with(stop_monitoring_query))
}
