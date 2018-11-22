extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate transpo_rt;
#[macro_use]
extern crate log;

use actix_web::{middleware, server, App};
use env_logger::{Builder, Env};
use std::sync::{Arc, Mutex};
use transpo_rt::gtfs_rt::{gtfs_rt, gtfs_rt_json};
use transpo_rt::state::State;

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let sys = actix::System::new("transpo-rt");

    let gtfs_rt_data = Arc::new(Mutex::new(None));
    server::new(move || {
        App::with_state(State {
            gtfs_rt: gtfs_rt_data.clone(),
        }).middleware(middleware::Logger::default())
        .resource("/gtfs_rt", |r| r.f(gtfs_rt))
        .resource("/gtfs_rt.json", |r| r.f(gtfs_rt_json))
    }).bind("127.0.0.1:8080")
    .unwrap()
    .start();

    info!("Started http server: 127.0.0.1:8080");
    let _ = sys.run();
}
