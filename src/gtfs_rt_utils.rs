use crate::context::{Context, DatedVehicleJourney, GtfsRT};
use crate::transit_realtime;
use actix_web::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use failure::format_err;
use failure::ResultExt;
use log::{debug, info, trace, warn};
use navitia_model::collection::Idx;
use navitia_model::objects::StopPoint;
use reqwest;
use std::collections::HashMap;
use std::io::Read;
use std::sync::MutexGuard;

const REFRESH_TIMEOUT_S: i64 = 60;

fn fetch_gtfs(url: &str) -> Result<Vec<u8>> {
    info!("fetching a gtfs_rt");
    reqwest::get(url)
        .and_then(|resp| resp.error_for_status())
        .map_err(|e| format_err!("Unable to fetch the gtfs RT {}", e))?
        .bytes()
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

pub fn update_gtfs_rt(context: &Context) -> Result<()> {
    let _guard = get_gtfs_rt(context)?;
    Ok(())
}

pub fn get_gtfs_rt(context: &Context) -> Result<MutexGuard<Option<GtfsRT>>> {
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

fn get_date_time(
    stop_time_event: &Option<transit_realtime::trip_update::StopTimeEvent>,
    timezone: chrono_tz::Tz,
) -> Option<NaiveDateTime> {
    stop_time_event
        .as_ref()
        .and_then(|ste| ste.time)
        .map(|t| DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(t, 0), Utc))
        .map(|utc_dt| utc_dt.with_timezone(&timezone))
        .map(|local_dt| local_dt.naive_local())
}

// Create the list of StopTimeUpdates from a gtfs-RT TripUpdate
//
// Note: we do not read the delay, we only read the updated time and compute the delay base on the scheduled time
// this reduce the problems when the GTFS-RT producer's data and our scheduled data are different
fn create_stop_time_updates(
    trip_update: &transit_realtime::TripUpdate,
    model: &navitia_model::Model,
    timezone: chrono_tz::Tz,
) -> Result<HashMap<u32, StopTimeUpdate>> {
    let mut res = HashMap::default();
    for stop_time_update in &trip_update.stop_time_update {
        let stop_sequence = stop_time_update.stop_sequence();
        let stop_id = stop_time_update.stop_id();
        let stop_idx = skip_fail!(model
            .stop_points
            .get_idx(&stop_id)
            .ok_or_else(|| format_err!(
                "impossible to find stop {} for vj {}",
                &stop_id,
                &trip_update.trip.trip_id()
            )));

        // first draft does not handle holes in the stoptimeupdates

        let updated_departure = get_date_time(&stop_time_update.departure, timezone);
        let updated_arrival = get_date_time(&stop_time_update.arrival, timezone);

        res.insert(
            stop_sequence,
            StopTimeUpdate {
                stop_point_idx: stop_idx,
                updated_departure,
                updated_arrival,
            },
        );
    }

    trace!(
        "trip {}, {} stop time events",
        &trip_update.trip.trip_id(),
        res.len()
    );
    Ok(res)
}

/// read a gtfs-rt FeedMessage to create a ModelUpdate,
/// a temporary structure used to
pub fn get_model_update(
    model: &navitia_model::Model,
    gtfs_rt: &transit_realtime::FeedMessage,
    timezone: chrono_tz::Tz,
) -> Result<ModelUpdate> {
    debug!("applying a trip update");
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
                    stop_time_update_by_sequence: create_stop_time_updates(tu, model, timezone)?,
                    update_dt: chrono::DateTime::<chrono::Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp(tu.timestamp.unwrap_or(0) as i64, 0),
                        chrono::Utc,
                    ),
                },
            );
        } else {
            debug!("unhandled feed entity: {}", &entity_id);
        }
    }

    debug!(
        "trip update applyed. {} trip updates",
        model_update.trips.len()
    );
    Ok(model_update)
}
