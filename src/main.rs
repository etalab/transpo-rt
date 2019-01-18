use actix_web::server;
use env_logger::{Builder, Env};
use failure::format_err;
use failure::ResultExt;
use std::path::PathBuf;
use structopt::StructOpt;
use transpo_rt::context::{DatasetInfo, Datasets};

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "transpo-rt")]
struct Params {
    #[structopt(
        short = "c",
        long = "config-file",
        help = "configurationfile",
        env = "TRANSPO_RT_CONFIG_FILE",
        parse(from_os_str)
    )]
    config_file: Option<PathBuf>,
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
}

/// Load datasets from the configuration
/// if a config file has been given, we get the dataset from here,
/// else we read the gtfs/url cli parameter to create a 'default' dataset with them
fn get_datasets(params: &Params) -> Result<Datasets, failure::Error> {
    if let Some(config) = &params.config_file {
        Ok(serde_yaml::from_reader(std::fs::File::open(config)?)
            .with_context(|e| format!("impossible to read config file because: {}", e))?)
    } else if let (Some(gtfs), Some(url)) = (&params.gtfs, &params.url) {
        Ok(Datasets {
            datasets: vec![DatasetInfo::new_default(gtfs, url)],
        })
    } else {
        Err(format_err!(
            "no config file nor gtfs/url given, impossible to start the api"
        ))
    }
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
