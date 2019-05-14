use actix_web::server;
use failure::format_err;
use failure::ResultExt;
use structopt::StructOpt;
use transpo_rt::datasets::{DatasetInfo, Datasets};

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
fn get_datasets(params: &Params) -> Result<Datasets, failure::Error> {
    if let Some(config) = &params.config_file {
        let yaml = if config.starts_with("http") {
            serde_yaml::from_reader(
                reqwest::get(config)
                    .with_context(|e| format!("impossible to read config url because: {}", e))?,
            )
        } else {
            serde_yaml::from_reader(
                std::fs::File::open(config)
                    .with_context(|e| format!("impossible to open config file because: {}", e))?,
            )
        };

        Ok(yaml.with_context(|e| format!("impossible to parse config file because: {}", e))?)
    } else if let (Some(gtfs), Some(url)) = (&params.gtfs, &params.url) {
        Ok(Datasets {
            datasets: vec![DatasetInfo::new_default(gtfs, &[url.clone()])],
        })
    } else {
        Err(format_err!(
            "no config file nor gtfs/url given, impossible to start the api"
        ))
    }
}

fn init_logger() -> (slog_scope::GlobalLoggerGuard, ()) {
    use slog::Drain;

    let drain = slog_term::FullFormat::new(slog_term::TermDecorator::new().stderr().build())
        .build()
        .fuse();

    let builder = slog_envlogger::LogBuilder::new(drain).filter(None, slog::FilterLevel::Info);
    let builder = match std::env::var("RUST_LOG") {
        Ok(s) => builder.parse(&s),
        _ => builder,
    };
    let drain = slog_async::Async::new(builder.build())
        .chan_size(512)
        .build();

    let log = slog::Logger::root(drain.fuse(), slog::slog_o!());
    let scope_guard = slog_scope::set_global_logger(log);
    let log_guard = slog_stdlog::init().unwrap();
    (scope_guard, log_guard)
}

fn main() {
    let (_log_guard, _) = init_logger();

    let params = Params::from_args();
    let sentry = sentry::init(params.sentry.clone().unwrap_or_else(|| "".to_owned()));
    if sentry.is_enabled() {
        log::info!("sentry activated");
        std::env::set_var("RUST_BACKTRACE", "1");
        sentry::integrations::panic::register_panic_handler();
    }

    let code = actix::System::run(move || {
        let bind = format!("{}:{}", &params.bind, &params.port);

        let today = chrono::Local::today(); //TODO use the timezone's dataset ?
        let period = transpo_rt::datasets::Period {
            begin: today.naive_local(),
            horizon: chrono::Duration::days(2),
        };
        let datasets_infos = get_datasets(&params).unwrap();

        let datasets_actors_addr = transpo_rt::server::create_all_actors(&datasets_infos, &period);
        server::new(move || {
            transpo_rt::server::create_server(&datasets_actors_addr, &datasets_infos)
        })
        .bind(bind)
        .unwrap()
        .start();
    });

    std::process::exit(code);
}
