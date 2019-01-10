use actix::Actor;
use actix_web::server;
use env_logger::{Builder, Env};
use std::sync::Arc;
use structopt::StructOpt;
use transpo_rt::context;
use transpo_rt::dataset_handler_actor;
use transpo_rt::update_actors;

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "transpo-rt")]
struct Params {
    #[structopt(
        short = "g",
        long = "gtfs",
        help = "path or url to the GTFS zip",
        env = "TRANSPO_RT_GTFS"
    )]
    gtfs: String,
    #[structopt(
        short = "u",
        long = "url",
        help = "URL to the GTFS-RT provider",
        env = "TRANSPO_RT_GTFS_RT_URL"
    )]
    url: String,
    #[structopt(
        short = "p",
        long = "port",
        help = "Port to listen to",
        env = "TRANSPO_RT_PORT",
        default_value = "8080"
    )]
    port: usize,
    #[structopt(
        short = "b",
        long = "bind",
        help = "Bind adress",
        env = "TRANSPO_RT_BIND",
        default_value = "0.0.0.0"
    )]
    bind: String,
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let code = actix::System::run(|| {
        let params = Params::from_args();
        let bind = format!("{}:{}", &params.bind, &params.port);

        let today = chrono::Local::today(); //TODO use the timezone's dataset ?
        let period = transpo_rt::context::Period {
            begin: today.naive_local(),
            end: today.succ().succ().naive_local(),
        };
        // todo better dataset to load construction
        let dataset_infos =
            transpo_rt::server::DatasetToLoad::new_default(&params.gtfs, &params.url);
        let dataset = transpo_rt::server::make_dataset(&dataset_infos, &period);
        let dataset_actors = dataset_handler_actor::DatasetActor {
            gtfs: Arc::new(dataset),
        };
        let dataset_actors_addr = dataset_actors.start();
        let base_schedule_reloader = update_actors::BaseScheduleReloader {
            feed_construction_info: context::FeedConstructionInfo {
                feed_path: dataset_infos.gtfs.clone(),
                generation_period: period.clone(),
            },
            dataset_actor: dataset_actors_addr.clone(),
        };
        base_schedule_reloader.start();
        server::new(move || transpo_rt::server::create_server(dataset_actors_addr.clone()))
            .bind(bind)
            .unwrap()
            .start();
    });

    std::process::exit(code);
}
