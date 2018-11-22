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
extern crate gtfs_structures;
extern crate serde;

pub mod transit_realtime {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

pub mod context;
pub mod gtfs_rt;
