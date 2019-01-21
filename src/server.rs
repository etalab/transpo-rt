use crate::actors::{BaseScheduleReloader, DatasetActor, RealTimeReloader};
use crate::context;
use crate::context::{Dataset, DatasetInfo, Datasets, Period};
use crate::routes::{
    gtfs_rt, gtfs_rt_json, list_datasets, sp_discovery, status_query, stop_monitoring_query,
};
use actix::Actor;
use actix::Addr;
use actix_web::middleware::cors::Cors;
use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{middleware, App};
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn create_dataset_actors(
    dataset_info: &DatasetInfo,
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

pub fn create_all_actors(
    datasets: &Datasets,
    generation_period: &Period,
) -> BTreeMap<String, Addr<DatasetActor>> {
    datasets
        .datasets
        .iter()
        .map(|d| (d.id.clone(), create_dataset_actors(&d, generation_period)))
        .collect()
}

pub fn create_datasets_servers(
    datasets_actors: &BTreeMap<String, Addr<DatasetActor>>,
) -> Vec<App<Addr<DatasetActor>>> {
    datasets_actors
        .iter()
        .map(|(id, a)| {
            App::with_state(a.clone())
                .prefix(format!("/{id}", id = &id))
                .middleware(middleware::Logger::default())
                .middleware(Cors::build().allowed_methods(vec!["GET"]).finish())
                .resource("/status", |r| r.f(status_query))
                .resource("/gtfs_rt", |r| r.f(gtfs_rt))
                .resource("/gtfs_rt.json", |r| r.f(gtfs_rt_json))
                .resource("/siri-lite/stoppoints_discovery.json", |r| {
                    r.with(sp_discovery)
                })
                .resource("/siri-lite/stop_monitoring.json", |r| {
                    r.with(stop_monitoring_query)
                })
        })
        .collect()
}

pub fn create_server(
    datasets_actors: &BTreeMap<String, Addr<DatasetActor>>,
    datasets: &Datasets,
) -> Vec<Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>>> {
    create_datasets_servers(datasets_actors)
        .into_iter()
        .map(|s| s.boxed())
        .chain(std::iter::once(
            App::with_state(datasets.clone())
                .middleware(middleware::Logger::default())
                .middleware(Cors::build().allowed_methods(vec!["GET"]).finish())
                .resource("/datasets", |r| r.f(list_datasets))
                .boxed(),
        ))
        .collect()
}
