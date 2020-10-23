use crate::datasets::DatedVehicleJourney;
use crate::transit_realtime;
use anyhow::anyhow;
use anyhow::Error;
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{debug, trace, warn};
use std::collections::HashMap;
use transit_model::collection::Idx;
use transit_model::objects::StopPoint;

#[derive(Debug, PartialEq, Eq)]
pub struct StopTimeUpdate {
    pub stop_point_idx: Option<Idx<StopPoint>>,
    pub updated_departure: Option<NaiveDateTime>,
    pub updated_arrival: Option<NaiveDateTime>,
}

pub struct TripUpdate {
    pub stop_time_update_by_sequence: HashMap<u32, StopTimeUpdate>,
    pub update_dt: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
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
    model: &transit_model::Model,
    timezone: chrono_tz::Tz,
) -> Result<HashMap<u32, StopTimeUpdate>, Error> {
    let mut res = HashMap::default();
    for stop_time_update in &trip_update.stop_time_update {
        let stop_sequence = skip_fail!(stop_time_update.stop_sequence.ok_or_else(|| anyhow!(
            "no stop_sequence provided, for the moment we don't handle this case"
        )));
        let stop_id = &stop_time_update.stop_id;

        let stop_idx = match stop_id
            .as_ref()
            .map(|stop_id| model.stop_points.get_idx(&stop_id))
        {
            Some(None) => {
                warn!(
                    "impossible to find stop {:?} for vj {}",
                    &stop_id,
                    &trip_update.trip.trip_id()
                );
                continue;
            }
            Some(Some(v)) => Some(v),
            None => None,
        };

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

fn default_date(timezone: chrono_tz::Tz) -> chrono::NaiveDate {
    chrono::Utc::now()
        .with_timezone(&timezone)
        .date()
        .naive_local()
}

fn get_date(
    trip: &transit_realtime::TripDescriptor,
    timezone: chrono_tz::Tz,
) -> Result<chrono::NaiveDate, anyhow::Error> {
    trip.start_date.as_ref().map_or_else(
        || Ok(default_date(timezone)),
        |s| {
            chrono::NaiveDate::parse_from_str(s, "%Y%m%d")
                .map_err(|e| anyhow!("Impossible to parse date: {}", e))
        },
    )
}

// TODO move this in transit_model ?
fn make_navitia_route_id(gtfs_route_id: &str, direction_id: u32) -> Result<String, anyhow::Error> {
    match direction_id {
        0 => Ok(gtfs_route_id.to_owned()),
        1 => Ok(format!("{}_R", gtfs_route_id)),
        n => Err(anyhow!("{} is not a valid GTFS direction", n)),
    }
}

// TODO move this in transit_model ?
fn find_corresponging_vjs(
    model: &transit_model::Model,
    gtfs_route_id: &str,
    direction_id: u32,
    start_date: chrono::NaiveDate,
    start_time: transit_model::objects::Time,
) -> Result<Vec<Idx<transit_model::objects::VehicleJourney>>, anyhow::Error> {
    let route_id = make_navitia_route_id(gtfs_route_id, direction_id)?;

    let route_idx = model
        .routes
        .get_idx(&route_id)
        .ok_or_else(|| anyhow!("impossible to find route {}", route_id))?;

    Ok(model
        .get_corresponding_from_idx(route_idx)
        .into_iter()
        .map(|vj_idx| (&model.vehicle_journeys[vj_idx], vj_idx))
        .filter(|(vj, _)| {
            // we want all the vjs that are valid this day
            model
                .calendars
                .get(&vj.service_id)
                .map(|service| service.dates.contains(&start_date))
                .unwrap_or(false)
        })
        .filter(|(vj, _)| {
            // and the trip should start at start_time
            vj.stop_times
                .get(0)
                .map(|st| st.departure_time == start_time)
                .unwrap_or(false)
        })
        .map(|(_, vj_idx)| vj_idx)
        .collect())
}

fn get_dated_vj(
    model: &transit_model::Model,
    trip: &transit_realtime::TripDescriptor,
    entity_id: &str,
    timezone: chrono_tz::Tz,
) -> Result<DatedVehicleJourney, anyhow::Error> {
    let vj_idx = model.vehicle_journeys.get_idx(trip.trip_id());

    let vj_idx = if let Some(vj_idx) = vj_idx {
        vj_idx
    } else {
        // we did not find the vj by it's id, we'll try to find it with the route_id
        if let (Some(route_id), Some(direction_id), Some(start_time)) =
            (&trip.route_id, trip.direction_id, &trip.start_time)
        {
            use std::str::FromStr;
            let date = get_date(&trip, timezone)?;
            let time = transit_model::objects::Time::from_str(start_time)?;
            let vjs = find_corresponging_vjs(model, &route_id, direction_id, date, time)?;

            match vjs.len() {
                1 => Ok(vjs[0]),
                0 => Err(anyhow!(
                    "for entity {}, impossible to find a matching trip",
                    &entity_id
                )),
                l => Err(anyhow!(
                    "for entity {}, there is no trip id, and {} matching trips, we can't choose one",
                    &entity_id,
                    l
                )),
            }
        } else {
            Err(anyhow!(
                "impossible to find trip {} for entity {} and no route_id was provided",
                &trip.trip_id(),
                &entity_id
            ))
        }?
    };

    let date = get_date(trip, timezone)?;

    Ok(DatedVehicleJourney { vj_idx, date })
}

/// read a gtfs-rt FeedMessage to create a ModelUpdate,
/// a temporary structure used to
pub fn get_model_update(
    model: &transit_model::Model,
    gtfs_rts: &[transit_realtime::FeedMessage],
    timezone: chrono_tz::Tz,
) -> Result<ModelUpdate, Error> {
    debug!("applying a trip update");
    let mut model_update = ModelUpdate::default();
    let mut unhandled_entities = 0;
    for gtfs_rt in gtfs_rts {
        for entity in &gtfs_rt.entity {
            let entity_id = &entity.id;
            if let Some(tu) = &entity.trip_update {
                let dated_vj = skip_fail!(get_dated_vj(&model, &tu.trip, entity_id, timezone));
                model_update.trips.insert(
                    dated_vj,
                    TripUpdate {
                        stop_time_update_by_sequence: create_stop_time_updates(
                            tu, model, timezone,
                        )?,
                        update_dt: chrono::DateTime::<chrono::Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp(
                                tu.timestamp.unwrap_or(0) as i64,
                                0,
                            ),
                            chrono::Utc,
                        ),
                    },
                );
            } else {
                unhandled_entities += 1;
            }
        }
    }

    debug!(
        "trip update applyed. {} trip updates",
        model_update.trips.len()
    );
    debug!("{} unhandled entities", unhandled_entities);
    Ok(model_update)
}

#[cfg(test)]
mod test {
    use crate::transit_realtime as tr;

    fn make_fake_model() -> transit_model::Model {
        transit_model_builder::ModelBuilder::default()
            .calendar("c", |c| {
                c.dates.insert(chrono::NaiveDate::from_ymd(2019, 2, 6));
            })
            .route("l1", |r| {
                r.name = "ligne 1".to_owned();
            })
            .route("l1_R", |r| {
                r.name = "ligne 1 backward".to_owned();
            })
            .vj("vj1", |vj_builder| {
                vj_builder
                    .route("l1")
                    .calendar("c")
                    .st("A", "10:00:00", "10:01:00")
                    .st("B", "11:00:00", "11:01:00")
                    .st("C", "12:00:00", "12:01:00");
            })
            .vj("vj2", |vj_builder| {
                vj_builder
                    .route("l1")
                    .calendar("c")
                    .st("B", "11:30:00", "11:31:00")
                    .st("D", "15:00:00", "15:01:00");
            })
            .build()
    }

    #[test]
    fn corresponding_vj_with_id() {
        let trip_descriptor = tr::TripDescriptor {
            trip_id: Some("vj1".to_owned()),
            start_date: Some("20190206".to_owned()),
            ..Default::default()
        };

        let model = make_fake_model();

        let dated_vj = super::get_dated_vj(&model, &trip_descriptor, "entity_id", chrono_tz::UTC);

        // we should be able to find the vj since the id is valid
        let vj_idx = dated_vj.unwrap().vj_idx;
        let vj = &model.vehicle_journeys[vj_idx];
        assert_eq!(&vj.id, "vj1");
    }

    #[test]
    fn corresponding_vj_without_id() {
        let trip_descriptor = tr::TripDescriptor {
            start_date: Some("20190206".to_owned()),
            ..Default::default()
        };
        let model = make_fake_model();
        let dated_vj = super::get_dated_vj(&model, &trip_descriptor, "entity_id", chrono_tz::UTC);
        // we shouldn't be able to find a vj
        assert_eq!(
            &format!("{}", dated_vj.unwrap_err()),
            "impossible to find trip  for entity entity_id and no route_id was provided"
        );
    }

    #[test]
    fn corresponding_vj_with_wrong_id() {
        let trip_descriptor = tr::TripDescriptor {
            trip_id: Some("id_that_does_not_exist".to_owned()),
            start_date: Some("20190206".to_owned()),
            ..Default::default()
        };
        let model = make_fake_model();
        let dated_vj = super::get_dated_vj(&model, &trip_descriptor, "entity_id", chrono_tz::UTC);
        // we shouldn't be able to find a vj
        assert_eq!(&format!("{}", dated_vj.unwrap_err()),
        "impossible to find trip id_that_does_not_exist for entity entity_id and no route_id was provided");
    }

    #[test]
    fn corresponding_vj_with_wrong_id_but_way_to_find_vj() {
        // the trip_id is not good, but with the route_id, the direction, and the start date time
        // we are able to identify an unique vj
        // cf: "Alternative trip matching"
        // in https://developers.google.com/transit/gtfs-realtime/guides/trip-updates
        let trip_descriptor = tr::TripDescriptor {
            trip_id: Some("id_that_does_not_exist".to_owned()),
            route_id: Some("l1".to_owned()),
            start_date: Some("20190206".to_owned()),
            start_time: Some("10:01:00".to_owned()),
            direction_id: Some(0),
            ..Default::default()
        };
        let model = make_fake_model();
        let dated_vj = super::get_dated_vj(&model, &trip_descriptor, "entity_id", chrono_tz::UTC);

        // we should be able to find the vj since the id is valid
        let vj_idx = dated_vj.unwrap().vj_idx;
        let vj = &model.vehicle_journeys[vj_idx];
        assert_eq!(&vj.id, "vj1");
    }

    #[test]
    fn corresponding_vj_with_wrong_id_but_no_way_to_find_vj() {
        // the trip_id is not good, but with the route_id
        // we are able to identify an unique vj
        // the problem is that this vj does not run on the given date, there is an error
        let trip_descriptor = tr::TripDescriptor {
            trip_id: Some("id_that_does_not_exist".to_owned()),
            route_id: Some("l1".to_owned()),
            start_date: Some("20190210".to_owned()),
            start_time: Some("10:01:00".to_owned()),
            direction_id: Some(0),
            ..Default::default()
        };
        let model = make_fake_model();
        let dated_vj = super::get_dated_vj(&model, &trip_descriptor, "entity_id", chrono_tz::UTC);
        assert_eq!(
            &format!("{}", dated_vj.unwrap_err()),
            "for entity entity_id, impossible to find a matching trip"
        );
    }

    #[test]
    fn corresponding_vj_with_wrong_id_but_too_many_matching_vj() {
        // the trip_id is not good, and we are able to match some vj,
        // but more than one, we cannot chose, there is an error
        let trip_descriptor = tr::TripDescriptor {
            trip_id: Some("id_that_does_not_exist".to_owned()),
            route_id: Some("l1".to_owned()),
            start_date: Some("20190206".to_owned()),
            start_time: Some("10:01:00".to_owned()),
            direction_id: Some(1), // 1 means backward direction, so in transit_model it will means route "l1_R"
            ..Default::default()
        };
        let model = transit_model_builder::ModelBuilder::default()
            .calendar("c", |c| {
                c.dates.insert(chrono::NaiveDate::from_ymd(2019, 2, 6));
            })
            .route("l1", |r| {
                r.name = "ligne 1".to_owned();
            })
            .route("l1_R", |r| {
                r.name = "ligne 1 backward".to_owned();
            })
            .vj("vj1", |vj_builder| {
                vj_builder
                    .route("l1_R")
                    .calendar("c")
                    .st("A", "10:00:00", "10:01:00")
                    .st("B", "11:00:00", "11:01:00")
                    .st("C", "12:00:00", "12:01:00");
            })
            .vj("vj2", |vj_builder| {
                vj_builder
                    .route("l1_R")
                    .calendar("c")
                    .st("A", "10:00:00", "10:01:00")
                    .st("B", "11:00:00", "11:01:00")
                    .st("C", "12:00:00", "12:01:00");
            })
            .build();
        let dated_vj = super::get_dated_vj(&model, &trip_descriptor, "entity_id", chrono_tz::UTC);
        // vj1 and vj2 are eligible, there is an error
        assert_eq!(
            &format!("{}", dated_vj.unwrap_err()),
            "for entity entity_id, there is no trip id, and 2 matching trips, we can\'t choose one"
        );
    }
}
