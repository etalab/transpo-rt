#[macro_use]
extern crate log;
#[macro_use]
extern crate prost_derive;
#[macro_use]
extern crate serde_derive;

pub mod transit_realtime {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

pub mod context;
pub mod gtfs_rt;
pub mod server;
pub mod siri_model;
pub mod stop_monitoring;
pub mod stoppoints_discovery;
