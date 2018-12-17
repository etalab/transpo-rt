use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use gtfs_structures;
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
pub struct Connection {
    pub vj_idx: Idx<navitia_model::objects::VehicleJourney>,
    pub stop_point_idx: Idx<navitia_model::objects::StopPoint>,
    pub dep_time: NaiveDateTime,
    pub arr_time: NaiveDateTime,
    pub sequence: u32,
}

pub struct Timetable {
    pub connections: Vec<Connection>,
}

pub struct Data {
    pub gtfs: gtfs_structures::Gtfs,
    pub raw: navitia_model::Model,
    pub timetable: Timetable,
}

#[derive(Clone)]
pub struct Context {
    pub gtfs_rt: Arc<Mutex<Option<GtfsRT>>>,
    pub gtfs_rt_provider_url: String,
    pub data: Arc<Mutex<Data>>,
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
    let date = if time.hours() > 24 { date.succ() } else { date };
    date.and_time(chrono::NaiveTime::from_hms(
        time.hours() % 24,
        time.minutes(),
        time.seconds(),
    ))
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
                    vj_idx,
                    stop_point_idx: st.stop_point_idx,
                    dep_time: create_dt(*date, st.departure_time),
                    arr_time: create_dt(*date, st.arrival_time),
                    sequence: st.sequence,
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
    pub fn new(
        gtfs: gtfs_structures::Gtfs,
        ntm: navitia_model::Model,
        generation_period: &Period,
    ) -> Self {
        Self {
            gtfs,
            timetable: create_timetable(&ntm, generation_period),
            raw: ntm,
        }
    }
}
