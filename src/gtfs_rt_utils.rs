use crate::context::{Context, GtfsRT};
use crate::transit_realtime;
use actix_web::Result;
use chrono::NaiveDateTime;
use chrono::Utc;
use failure::Error;
use log::info;
use navitia_model::collection::Idx;
use navitia_model::objects::{StopPoint, VehicleJourney};
use reqwest;
use std::collections::HashMap;
use std::io::Read;
use std::sync::MutexGuard;

const REFRESH_TIMEOUT_S: i64 = 60;

fn fetch_gtfs(url: &str) -> Result<Vec<u8>, Error> {
    info!("fetching a gtfs_rt");
    let pbf = reqwest::get(url)?.error_for_status()?;

    pbf.bytes()
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| e.into())
}

fn refresh_needed(previous: &Option<GtfsRT>) -> bool {
    previous
        .as_ref()
        .map(|g| g.datetime)
        .map(|dt| (chrono::Utc::now() - dt).num_seconds().abs() > REFRESH_TIMEOUT_S)
        .unwrap_or(true)
}

pub fn update_gtfs_rt(context: &Context) -> Result<(), Error> {
    let _guard = get_gtfs_rt(context)?;
    Ok(())
}

pub fn get_gtfs_rt(context: &Context) -> Result<MutexGuard<Option<GtfsRT>>, Error> {
    let mut saved_data = context.gtfs_rt.lock().unwrap();
    if refresh_needed(&saved_data) {
        *saved_data = Some(GtfsRT {
            data: fetch_gtfs(&context.gtfs_rt_provider_url)?,
            datetime: Utc::now(),
        });
    }
    Ok(saved_data)
}

pub struct StopTimeUpdate {
    pub stop_point_idx: Idx<StopPoint>,
    pub updated_departure: Option<NaiveDateTime>,
    pub updated_arrival: Option<NaiveDateTime>,
}

pub struct TripUpdate {
    pub stop_time_update_by_sequence: HashMap<u32, StopTimeUpdate>,
    pub update_dt: chrono::DateTime<chrono::Utc>,
    pub date: chrono::NaiveDate,
}

pub struct ModelUpdate {
    pub trips: HashMap<Idx<VehicleJourney>, TripUpdate>,
}

pub fn get_model_update(
    model: &navitia_model::Model,
    gtfs_rt: &transit_realtime::FeedMessage,
) -> Result<ModelUpdate> {
    unimplemented!()
}
