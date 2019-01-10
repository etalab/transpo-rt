use crate::context::{Data, Dataset, FeedConstructionInfo, Period};
use crate::dataset_handler_actor::DatasetActor;
use crate::gtfs_rt::{gtfs_rt, gtfs_rt_json};
use crate::status::status_query;
use crate::stop_monitoring::stop_monitoring_query;
use crate::stoppoints_discovery::sp_discovery;
use actix::Addr;
use actix_web::middleware::cors::Cors;
use actix_web::{middleware, App};
use std::sync::Mutex;

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

pub fn make_dataset(dataset: &DatasetToLoad, generation_period: &Period) -> Dataset {
    let gtfs = &dataset.gtfs;
    let url = &dataset.gtfs_rt;
    let data = Data::from_path(gtfs, generation_period);
    Dataset {
        gtfs_rt: Mutex::new(None),
        data: Mutex::new(data),
        gtfs_rt_provider_url: url.to_owned(),
        feed_construction_info: FeedConstructionInfo {
            feed_path: gtfs.to_owned(),
            generation_period: generation_period.clone(),
        },
    }
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
