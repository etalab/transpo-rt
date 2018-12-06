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
        help = "path to the GTFS zip",
        env = "TRANSPO_RT_GTFS",
    )]
    gtfs: PathBuf,
    #[structopt(
        short = "u",
        long = "url",
        help = "URL to the GTFS-RT provider",
        env = "TRANSPO_RT_GTFS_RT_URL",
    )]
    url: String,
    #[structopt(
        short = "p",
        long = "port",
        help = "Port to listen to",
        env = "TRANSPO_RT_PORT",
        default_value = "8080",
    )]
    port: usize,
    #[structopt(
        short = "b",
        long = "bind",
        help = "Bind adress",
        env = "TRANSPO_RT_BIND",
        default_value = "0.0.0.0",
    )]
    bind: String,
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let sys = actix::System::new("transpo-rt");
    let params = Params::from_args();
    let bind = format!("{}:{}", &params.bind, &params.port);
    server::new(move || transpo_rt::server::create_server(&params.gtfs, &params.url))
        .bind(bind)
        .unwrap()
        .start();

    info!("Started http server: 127.0.0.1:8080");
    let _ = sys.run();
}
