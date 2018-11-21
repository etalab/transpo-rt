use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct GtfsRT {
    pub datetime: DateTime<Utc>,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct State {
    pub gtfs_rt: Arc<Mutex<Option<GtfsRT>>>,
}
