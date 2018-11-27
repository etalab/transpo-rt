use gtfs_structures;

use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct GtfsRT {
    pub datetime: DateTime<Utc>,
    pub data: Vec<u8>,
}

pub struct Context {
    pub gtfs_rt: Arc<Mutex<Option<GtfsRT>>>,
    pub gtfs: gtfs_structures::Gtfs,
    pub gtfs_rt_provider_url: String,
}
