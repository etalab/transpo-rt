extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate transpo_rt;
#[macro_use]
extern crate log;
extern crate gtfs_structures;
extern crate structopt;

use actix_web::server;
use env_logger::{Builder, Env};
use std::path::PathBuf;
use structopt::StructOpt;

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
    server::new(move || {
        let params = Params::from_args();
        transpo_rt::server::create_server(&params.gtfs, params.url)
    }).bind("127.0.0.1:8080")
    .unwrap()
    .start();

    info!("Started http server: 127.0.0.1:8080");
    let _ = sys.run();
}
