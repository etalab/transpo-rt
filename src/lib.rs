extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate log;
extern crate prost;
extern crate reqwest;
#[macro_use]
extern crate prost_derive;
#[macro_use]
extern crate serde_derive;
extern crate bytes;
extern crate serde;

pub mod transit_realtime {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

pub mod gtfs_rt;
pub mod state;

pub use gtfs_rt::{gtfs_rt, gtfs_rt_json};
pub use state::State;
