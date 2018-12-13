use chrono::{DateTime, Utc};
use gtfs_structures;
use navitia_model::collection::Idx;
use std::sync::Arc;
use std::sync::Mutex;

pub enum Stop {
    StopPoint(Idx<navitia_model::objects::StopPoint>),
    StopArea(Idx<navitia_model::objects::StopArea>),
}

#[derive(Clone)]
pub struct GtfsRT {
    pub datetime: DateTime<Utc>,
    pub data: Vec<u8>,
}

pub struct Data {
    pub gtfs: gtfs_structures::Gtfs,
    pub raw: navitia_model::Model,
}

#[derive(Clone)]
pub struct Context {
    pub gtfs_rt: Arc<Mutex<Option<GtfsRT>>>,
    pub gtfs_rt_provider_url: String,
    pub data: Arc<Mutex<Data>>,
}

impl Data {
    pub fn new(gtfs: gtfs_structures::Gtfs, ntfs: navitia_model::Model) -> Self {
        Self { gtfs, raw: ntfs }
    }
}
