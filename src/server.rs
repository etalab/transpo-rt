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

fn create_dataset_actors(
    dataset_info: &DatasetInfo,
    generation_period: &Period,
) -> Result<Addr<DatasetActor>, failure::Error> {
    let logger = slog_scope::logger().new(slog::o!("instance" => dataset_info.id.clone()));
    slog_scope::scope(&logger, || {
        log::info!("creating actors");
        let dataset = Dataset::try_from_dataset_info(dataset_info.clone(), &generation_period)?;
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
        realtime_reloader.start();

        Ok(dataset_actors_addr)
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
        .filter_map(|(id, d)| match d {
            Ok(d) => Some((id, d)),
            Err(e) => {
                // log on sentry
                let msg = format!("impossible to create dataset {}: {}", id, e);
                sentry::capture_message(&msg, sentry::Level::Error);
                log::error!("{}", &msg);
                None
            }
        })
        .collect()
}

fn register_dataset_routes(
    cfg: &mut web::ServiceConfig,
    datasets_actors: &BTreeMap<String, Addr<DatasetActor>>,
) {
    for (id, dataset_actor) in datasets_actors {
        log::info!("adding route for {}", id);
        cfg.service(
            web::scope(&format!("/{id}", id = &id))
                .data(dataset_actor.clone())
                .service(status_query)
                .service(gtfs_rt_protobuf)
                .service(gtfs_rt_json)
                .service(siri_endpoint)
                .service(stoppoints_discovery_query)
                .service(stop_monitoring_query)
                .service(general_message_query),
        );
    }
}

pub fn init_routes(
    cfg: &mut web::ServiceConfig,
    datasets_actors: &BTreeMap<String, Addr<DatasetActor>>,
    datasets: &Datasets,
) {
    log::info!("creating default routes");
    cfg.data(datasets.clone())
        .service(documentation)
        .service(entry_point);
    log::info!("creating dataset routes {}", datasets_actors.len());
    register_dataset_routes(cfg, datasets_actors);
}
