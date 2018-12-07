extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate gtfs_structures;
extern crate log;
extern crate structopt;
extern crate transpo_rt;

use actix_web::server;
use env_logger::{Builder, Env};
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "transpo-rt")]
struct Params {
    #[structopt(
        short = "g",
        long = "gtfs",
        help = "path or url to the GTFS zip",
        env = "TRANSPO_RT_GTFS",
    )]
    gtfs: String,
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
    let context = transpo_rt::server::make_context(&params.gtfs, &params.url);
    server::new(move || transpo_rt::server::create_server(context.clone()))
        .bind(bind)
        .unwrap()
        .start();

    let _ = sys.run();
}
