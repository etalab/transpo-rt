use anyhow::{anyhow, Context};
use structopt::StructOpt;
use transpo_rt::datasets::{DatasetInfo, Datasets};
use transpo_rt::middlewares;

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "transpo-rt")]
struct Params {
    #[structopt(
        short = "c",
        long = "config-file",
        help = "path or url to configuration yaml file",
        env = "TRANSPO_RT_CONFIG_FILE"
    )]
    config_file: Option<String>,
    #[structopt(
        short = "g",
        long = "gtfs",
        help = "path or url to the GTFS zip. Note: if a config file has been given, this option is not taken into account.",
        env = "TRANSPO_RT_GTFS"
    )]
    gtfs: Option<String>,
    #[structopt(
        short = "u",
        long = "url",
        help = "URL to the GTFS-RT provider. Note: if a config file has been given, this option is not taken into account",
        env = "TRANSPO_RT_GTFS_RT_URL"
    )]
    url: Option<String>,
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
    #[structopt(long = "sentry", help = "sentry dsn", env = "TRANSPO_RT_SENTRY")]
    sentry: Option<String>,
}

/// Load datasets from the configuration
/// if a config file has been given, we get the dataset from here,
/// else we read the gtfs/url cli parameter to create a 'default' dataset with them
fn get_datasets(params: &Params) -> Result<Datasets, anyhow::Error> {
    if let Some(config) = &params.config_file {
        let yaml = if config.starts_with("http") {
            serde_yaml::from_reader(
                reqwest::blocking::get(config)
                    .with_context(|| format!("impossible to read config url"))?,
            )
        } else {
            serde_yaml::from_reader(
                std::fs::File::open(config)
                    .with_context(|| format!("impossible to open config file",))?,
            )
        };

        Ok(yaml.with_context(|| format!("impossible to parse config file"))?)
    } else if let (Some(gtfs), Some(url)) = (&params.gtfs, &params.url) {
        Ok(Datasets {
            datasets: vec![DatasetInfo::new_default(gtfs, &[url.clone()])],
        })
    } else {
        Err(anyhow!(
            "no config file nor gtfs/url given, impossible to start the api"
        ))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _log_guard = transpo_rt::utils::init_logger();

    let params = Params::from_args();
    let sentry = sentry::init(params.sentry.clone().unwrap_or_else(|| "".to_owned()));
    if sentry.is_enabled() {
        log::info!("sentry activated");
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let bind = format!("{}:{}", &params.bind, &params.port);
    let today = chrono::Local::today(); //TODO use the timezone's dataset ?
    let period = transpo_rt::datasets::Period {
        begin: today.naive_local(),
        horizon: chrono::Duration::days(2),
    };
    let datasets_infos = get_datasets(&params).unwrap();
    // we create all the actors
    // this is an async function as we need to wait for all data (and realtime data too) to be read
    // we wait for this to be finished before spawning the webserver
    let actors = transpo_rt::server::create_all_actors(&datasets_infos, &period).await;

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_cors::Cors::default().allowed_methods(vec!["GET"]))
            .wrap_fn(middlewares::sentry::sentry_middleware)
            .wrap(actix_web::middleware::Logger::default())
            .configure(|cfg| transpo_rt::server::init_routes(cfg, &actors, &datasets_infos))
    })
    .bind(bind)?
    .run()
    .await
}
