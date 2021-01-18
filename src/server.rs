use crate::actors::{BaseScheduleReloader, DatasetActor, RealTimeReloader};
use crate::datasets;
use crate::datasets::{Dataset, DatasetInfo, Datasets, Period};
use crate::routes::{
    documentation, entry_point, general_message_query, gtfs_rt_json, gtfs_rt_protobuf,
    siri_endpoint, status_query, stop_monitoring_query, stoppoints_discovery_query,
};
use actix::{Actor, Addr};
use actix_web::web;
use std::collections::BTreeMap;
use std::sync::Arc;

async fn create_dataset_actors_impl(
    dataset_info: DatasetInfo,
    generation_period: &Period,
    logger: &slog::Logger,
) -> (DatasetInfo, Result<Addr<DatasetActor>, anyhow::Error>) {
    log::info!("creating actors");
    let dataset = Dataset::try_from_dataset_info(dataset_info.clone(), &generation_period);

    let arc_dataset = Arc::new(dataset);
    let rt_dataset =
        datasets::RealTimeDataset::new(arc_dataset.clone(), &dataset_info.gtfs_rt_urls);
    let dataset_actors = DatasetActor {
        gtfs: arc_dataset,
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
    // we fetch a first time the gtfs_rt feeds, for them to be available on startup
    realtime_reloader.update_realtime_data().await;
    realtime_reloader.start();

    (dataset_info, Ok(dataset_actors_addr))
}

async fn create_dataset_actors(
    dataset_info: DatasetInfo,
    generation_period: &Period,
) -> (DatasetInfo, Result<Addr<DatasetActor>, anyhow::Error>) {
    use slog_scope_futures::FutureExt;
    let logger = slog_scope::logger().new(slog::o!("instance" => dataset_info.id.clone()));
    create_dataset_actors_impl(dataset_info, generation_period, &logger)
        .with_logger(&logger)
        .await
}

pub async fn create_all_actors(
    datasets: Datasets,
    generation_period: &Period,
) -> BTreeMap<DatasetInfo, Addr<DatasetActor>> {
    let actors = datasets
        .datasets
        .into_iter()
        .map(|d| create_dataset_actors(d, &generation_period));

    async move {
        futures::future::join_all(actors)
            .await
            .into_iter()
            .filter_map(|(d, r)| match r {
                Ok(a) => Some((d, a)),
                Err(e) => {
                    // the invalid datasets are filtered
                    let msg = format!("impossible to create dataset {}: {}", &d.id, e);
                    sentry::capture_message(&msg, sentry::Level::Error);
                    log::error!("{}", &msg);
                    None
                }
            })
            .collect()
    }
    .await
}

fn register_dataset_routes(
    cfg: &mut web::ServiceConfig,
    datasets_actors: &BTreeMap<DatasetInfo, Addr<DatasetActor>>,
) {
    for (d, dataset_actor) in datasets_actors {
        cfg.service(
            web::scope(&format!("/{id}", id = &d.id))
                .data(dataset_actor.clone())
                .service(
                    web::resource("/")
                        .name(&format!("{}/status_query", &d.id))
                        .route(web::get().to(status_query)),
                )
                .service(
                    web::resource("/gtfs-rt/")
                        .name(&format!("{}/gtfs_rt_protobuf", &d.id))
                        .route(web::get().to(gtfs_rt_protobuf)),
                )
                .service(
                    web::resource("/gtfs-rt.json/")
                        .name(&format!("{}/gtfs_rt_json", &d.id))
                        .route(web::get().to(gtfs_rt_json)),
                )
                .service(
                    web::resource("/siri/2.0/")
                        .name(&format!("{}/siri_endpoint", &d.id))
                        .route(web::get().to(siri_endpoint)),
                )
                .service(
                    web::resource("/siri/2.0/stoppoints-discovery.json/")
                        .name(&format!("{}/stoppoints_discovery_query", &d.id))
                        .route(web::get().to(stoppoints_discovery_query)),
                )
                .service(
                    web::resource("/siri/2.0/stop-monitoring.json/")
                        .name(&format!("{}/stop_monitoring_query", &d.id))
                        .route(web::get().to(stop_monitoring_query)),
                )
                .service(
                    web::resource("/siri/2.0/general-message.json/")
                        .name(&format!("{}/general_message_query", &d.id))
                        .route(web::get().to(general_message_query)),
                ),
        );
    }
}

pub fn init_routes(
    cfg: &mut web::ServiceConfig,
    datasets_actors: &BTreeMap<DatasetInfo, Addr<DatasetActor>>,
) {
    let datasets = Datasets {
        datasets: datasets_actors.keys().cloned().collect(),
    };
    cfg.data(datasets)
        .service(documentation)
        .service(entry_point);
    register_dataset_routes(cfg, datasets_actors);
}
