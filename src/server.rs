use crate::actors::{BaseScheduleReloader, DatasetActor, RealTimeReloader};
use crate::datasets;
use crate::datasets::{Dataset, DatasetInfo, Datasets, Period};
use crate::routes::{
    api_entry_point, documentation, general_message_query, gtfs_rt, gtfs_rt_json, sp_discovery,
    status_query, stop_monitoring_query,
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
    let logger = slog_scope::logger().new(slog::o!("instance" => dataset_info.id.clone()));
    slog_scope::scope(&logger, || {
        log::info!("creating actors");
        let dataset =
            Dataset::try_from_dataset_info(dataset_info.clone(), &generation_period).unwrap();
        let arc_dataset = Arc::new(dataset);
        let rt_dataset =
            datasets::RealTimeDataset::new(arc_dataset.clone(), &dataset_info.gtfs_rt_urls);
        let dataset_actors = DatasetActor {
            gtfs: arc_dataset.clone(),
            realtime: Arc::new(rt_dataset),
        };
        let dataset_actors_addr = dataset_actors.start();
        let base_schedule_reloader = BaseScheduleReloader {
            feed_construction_info: datasets::FeedConstructionInfo {
                dataset_info: dataset_info.clone(),
                generation_period: generation_period.clone(),
            },
            dataset_actor: dataset_actors_addr.clone(),
            log: logger.clone(),
        };
        base_schedule_reloader.start();
        let realtime_reloader = RealTimeReloader {
            dataset_id: dataset_info.id.clone(),
            gtfs_rt_urls: dataset_info.gtfs_rt_urls.clone(),
            dataset_actor: dataset_actors_addr.clone(),
            log: logger.clone(),
        };
        realtime_reloader.start();

        dataset_actors_addr
    })
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
                .middleware(sentry_actix::SentryMiddleware::new())
                .middleware(Cors::build().allowed_methods(vec!["GET"]).finish())
                .resource("/", |r| {
                    r.name("dataset");
                    r.f(status_query)
                })
                .resource("/gtfs-rt", |r| {
                    r.name("gtfs-rt");
                    r.f(gtfs_rt)
                })
                .resource("/gtfs-rt.json", |r| {
                    r.name("gtfs-rt.json");
                    r.f(gtfs_rt_json)
                })
                .scope("/siri/2.0", |scope| {
                    scope
                        .resource("/stoppoints-discovery.json", |r| {
                            r.name("stoppoints-discovery");
                            r.with(sp_discovery)
                        })
                        .resource("/stop-monitoring.json", |r| {
                            r.name("stop-monitoring");
                            r.with(stop_monitoring_query)
                        })
                        .resource("/general-message.json", |r| {
                            r.name("general-message");
                            r.with_async(general_message_query)
                        })
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
        .map(actix_web::App::boxed)
        .chain(std::iter::once(
            App::with_state(datasets.clone())
                .middleware(middleware::Logger::default())
                .middleware(Cors::build().allowed_methods(vec!["GET"]).finish())
                .resource("/", |r| {
                    r.name("entrypoint");
                    r.f(api_entry_point)
                })
                .resource("/spec", |r| {
                    r.name("documentation");
                    r.f(documentation)
                })
                .boxed(),
        ))
        .collect()
}
