#[macro_use]
extern crate prost_derive;
#[macro_use]
extern crate serde_derive;

pub mod transit_realtime {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}
#[macro_use]
pub(crate) mod utils;

pub type Result<T> = std::result::Result<T, failure::Error>;

pub mod context;
pub mod gtfs_rt;
pub(crate) mod gtfs_rt_utils;
pub mod server;
pub mod siri_model;
pub mod stop_monitoring;
pub mod stoppoints_discovery;
