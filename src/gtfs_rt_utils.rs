use crate::context::{Context, DatedVehicleJourney, GtfsRT};
use crate::transit_realtime;
use actix_web::Result;
use chrono::NaiveDateTime;
use chrono::Utc;
use failure::format_err;
use failure::Error;
use failure::ResultExt;
use log::{debug, info, warn};
use navitia_model::collection::Idx;
use navitia_model::objects::StopPoint;
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
}

pub struct ModelUpdate {
    pub trips: HashMap<DatedVehicleJourney, TripUpdate>,
}

fn create_stop_time_updates(
    _trip_update: &transit_realtime::TripUpdate,
) -> Result<HashMap<u32, StopTimeUpdate>> {
    unimplemented!()
}

pub fn get_model_update(
    model: &navitia_model::Model,
    gtfs_rt: &transit_realtime::FeedMessage,
) -> Result<ModelUpdate> {
    let mut model_update = ModelUpdate {
        trips: HashMap::new(),
    };

    for entity in &gtfs_rt.entity {
        let entity_id = &entity.id;
        if let Some(tu) = &entity.trip_update {
            let trip_id = tu.trip.trip_id();
            let date = skip_fail!(tu
                .trip
                .start_date
                .as_ref()
                .ok_or_else(|| format_err!(
                    "The date is a mandatory field to apply a trip update, cannot apply entity {}",
                    &entity_id
                ))
                .and_then(
                    |date_str| Ok(
                        chrono::NaiveDate::parse_from_str(&date_str, "%Y%m%d").context(format!(
                            "impossible to read date from entity {}",
                            &entity_id
                        ))?
                    )
                ));

            let vj_idx =
                skip_fail!(model
                    .vehicle_journeys
                    .get_idx(trip_id)
                    .ok_or_else(|| format_err!(
                        "impossible to find trip {} for entity {}",
                        &trip_id,
                        &entity_id
                    )));

            let dated_vj = DatedVehicleJourney { vj_idx, date };
            model_update.trips.insert(
                dated_vj,
                TripUpdate {
                    stop_time_update_by_sequence: create_stop_time_updates(tu)?,
                    update_dt: chrono::DateTime::<chrono::Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp(tu.timestamp.unwrap_or(0) as i64, 0),
                        chrono::Utc,
                    ),
                },
            );
        } else {
            debug!("unhandled feed entity");
        }
    }
    Ok(model_update)
}
