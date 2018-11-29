extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate transpo_rt;
#[macro_use]
extern crate log;
extern crate gtfs_structures;
extern crate structopt;

use actix_web::{middleware, server, App};
use env_logger::{Builder, Env};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use transpo_rt::context::{lines_of_stop, Context};
use transpo_rt::gtfs_rt::{gtfs_rt, gtfs_rt_json};
use transpo_rt::stoppoints_discovery::stoppoints_discovery;

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "transpo-rt")]
struct Params {
    #[structopt(
        short = "g",
        long = "gtfs",
        parse(from_os_str),
        help = "path to the GTFS zip"
    )]
    gtfs: PathBuf,

    #[structopt(
        short = "u",
        long = "url",
        help = "URL to the GTFS-RT provider"
    )]
    url: String,
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let sys = actix::System::new("transpo-rt");
    let gtfs_rt_data = Arc::new(Mutex::new(None));

    server::new(move || {
        let params = Params::from_args();
        let gtfs = gtfs_structures::Gtfs::from_zip(params.gtfs.to_str().unwrap()).unwrap();
        App::with_state(Context {
            gtfs_rt: gtfs_rt_data.clone(),
            lines_of_stops: gtfs
                .stops
                .values()
                .map(|stop| (stop.id.to_owned(), lines_of_stop(&gtfs, stop)))
                .collect(),
            gtfs,
            gtfs_rt_provider_url: params.url,
        }).middleware(middleware::Logger::default())
        .resource("/gtfs_rt", |r| r.f(gtfs_rt))
        .resource("/gtfs_rt.json", |r| r.f(gtfs_rt_json))
        .resource("/stoppoints_discovery.json", |r| {
            r.with(stoppoints_discovery)
        })
    }).bind("127.0.0.1:8080")
    .unwrap()
    .start();

    info!("Started http server: 127.0.0.1:8080");
    let _ = sys.run();
}
