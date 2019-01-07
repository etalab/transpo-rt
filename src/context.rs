use actix::Actor;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use log::info;
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

#[derive(Clone, Debug)]
pub enum ScheduleRelationship {
    Scheduled,
    Skipped,
    NoData,
}

#[derive(Clone, Debug)]
pub struct RealTimeConnection {
    pub dep_time: Option<NaiveDateTime>,
    pub arr_time: Option<NaiveDateTime>,
    pub schedule_relationship: ScheduleRelationship,
    //TODO handle uncertainty
    pub update_time: chrono::DateTime<chrono::Utc>, //TODO move it to have one update_time for a trip, not one by stop_time
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct DatedVehicleJourney {
    pub vj_idx: Idx<navitia_model::objects::VehicleJourney>,
    pub date: chrono::NaiveDate,
}

#[derive(Clone, Debug)]
pub struct Connection {
    pub dated_vj: DatedVehicleJourney,
    pub stop_point_idx: Idx<navitia_model::objects::StopPoint>,
    pub dep_time: NaiveDateTime,
    pub arr_time: NaiveDateTime,
    pub sequence: u32,
    pub realtime_info: Option<RealTimeConnection>,
}

pub struct Timetable {
    pub connections: Vec<Connection>,
}

pub struct Data {
    pub ntm: navitia_model::Model,
    pub timetable: Timetable,
    pub timezone: Tz,
}

#[derive(Clone)]
pub struct Context {
    pub gtfs_rt: Arc<Mutex<Option<GtfsRT>>>,
    pub gtfs_rt_provider_url: String,
    pub data: Arc<Mutex<Data>>,
}

impl Actor for Context {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Starting the context actor");
    }
}

#[derive(Debug)]
pub struct Period {
    pub begin: NaiveDate,
    pub end: NaiveDate,
}

impl Period {
    pub fn contains(&self, date: NaiveDate) -> bool {
        self.begin <= date && date < self.end
    }
}

// create a dt from a Date and a StopTime's time
// Note: the time might be on the next day, for example "26:00:00"
// is the next day at 2 in the morning
fn create_dt(date: NaiveDate, time: navitia_model::objects::Time) -> NaiveDateTime {
    date.and_time(chrono::NaiveTime::from_hms(0, 0, 0))
        + chrono::Duration::seconds(i64::from(time.total_seconds()))
}

fn create_timetable(ntm: &navitia_model::Model, generation_period: &Period) -> Timetable {
    info!("computing timetable for {:?}", &generation_period);
    let begin_dt = Utc::now();
    let mut timetable = Timetable {
        connections: vec![],
    };
    for (vj_idx, vj) in ntm.vehicle_journeys.iter() {
        let service = ntm.calendars.get(&vj.service_id).unwrap();
        for st in &vj.stop_times {
            for date in service
                .dates
                .iter()
                .filter(|date| generation_period.contains(**date))
            {
                timetable.connections.push(Connection {
                    dated_vj: DatedVehicleJourney {
                        vj_idx,
                        date: *date,
                    },
                    stop_point_idx: st.stop_point_idx,
                    dep_time: create_dt(*date, st.departure_time),
                    arr_time: create_dt(*date, st.arrival_time),
                    sequence: st.sequence,
                    realtime_info: None,
                });
            }
        }
    }
    timetable.connections.sort_by_key(|a| a.dep_time);

    info!(
        "timetable of {} elements computed in {}",
        timetable.connections.len(),
        Utc::now().signed_duration_since(begin_dt)
    );

    timetable
}

impl Data {
    pub fn new(ntm: navitia_model::Model, generation_period: &Period) -> Self {
        // To correctly handle GTFS-RT stream we need the dataset's timezone,
        // as all the time in the dataset are in local time and the GTFS-RT gives its time
        // as UTC.
        // We consider that there can be at most one Company (gtfs's agency) in the dataset
        // and we consider that the dataset's timezone is it's agency's timezone
        let timezone = ntm
            .networks
            .values()
            .next()
            .and_then(|n| n.timezone.as_ref())
            .and_then(|t| {
                t.parse()
                    .map_err(|e| log::warn!("impossible to parse timezone {} because: {}", t, e))
                    .ok()
            })
            .expect("no timezone found, we will not be able to understand realtime information");

        Self {
            timetable: create_timetable(&ntm, generation_period),
            ntm,
            timezone,
        }
    }
}
