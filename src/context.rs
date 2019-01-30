use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use log::info;
use navitia_model::collection::Idx;
use std::collections::HashMap;
use std::sync::Arc;

use crate::transit_realtime;

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
}

pub struct Timetable {
    pub connections: Vec<Connection>,
}

#[derive(Default)]
pub struct UpdatedTimetable {
    /// the key is the index in the BaseSchedule connections Vector
    /// TODO: could we stronger type this index ?
    pub realtime_connections: HashMap<usize, RealTimeConnection>,
}

#[derive(Clone)]
pub struct FeedConstructionInfo {
    pub feed_path: String,
    pub generation_period: Period,
}

pub struct Dataset {
    pub ntm: navitia_model::Model,
    pub timetable: Timetable,
    pub timezone: Tz,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    pub feed_construction_info: FeedConstructionInfo,
}

pub struct RealTimeDataset {
    /// shared ptr to the base schedule dataset
    pub base_schedule_dataset: Arc<Dataset>,
    pub gtfs_rt: Option<GtfsRT>,
    pub gtfs_rt_provider_urls: Vec<String>,
    pub updated_timetable: UpdatedTimetable,
}

impl RealTimeDataset {
    pub fn new(base: Arc<Dataset>, urls: &[String]) -> Self {
        RealTimeDataset {
            base_schedule_dataset: base,
            gtfs_rt: None,
            gtfs_rt_provider_urls: urls.to_owned(),
            updated_timetable: UpdatedTimetable::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Period {
    pub begin: NaiveDate,
    pub horizon: chrono::Duration,
}

#[derive(Deserialize, Clone)]
pub struct Datasets {
    pub datasets: Vec<DatasetInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct DatasetInfo {
    pub name: String,
    pub id: String,
    pub gtfs: String,
    pub gtfs_rt_urls: Vec<String>,
}

impl DatasetInfo {
    pub fn new_default(gtfs: &str, gtfs_rt_urls: &[String]) -> Self {
        Self {
            id: "default".into(),
            name: "default name".into(),
            gtfs: gtfs.to_owned(),
            gtfs_rt_urls: gtfs_rt_urls.to_vec(),
        }
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
    let begin = generation_period.begin;
    let end = begin + generation_period.horizon;

    for (vj_idx, vj) in ntm.vehicle_journeys.iter() {
        let service = ntm.calendars.get(&vj.service_id).unwrap();
        for st in &vj.stop_times {
            for date in service
                .dates
                .iter()
                .filter(|date| **date >= begin)
                .filter(|date| **date < end)
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

impl GtfsRT {
    pub fn decode_feed_message(&self) -> Option<transit_realtime::FeedMessage> {
        use bytes::IntoBuf;
        use prost::Message;
        transit_realtime::FeedMessage::decode(self.data.clone().into_buf())
            .map_err(|e| log::warn!("Unable to decode feed message, {}", e))
            .ok()
    }
}

impl Dataset {
    pub fn new(ntm: navitia_model::Model, gtfs_path: &str, generation_period: &Period) -> Self {
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
            loaded_at: chrono::Utc::now(),
            feed_construction_info: FeedConstructionInfo {
                feed_path: gtfs_path.to_owned(),
                generation_period: generation_period.clone(),
            },
        }
    }

    pub fn from_path(gtfs: &str, generation_period: &Period) -> Self {
        log::info!("reading from path");
        let nav_data = if gtfs.starts_with("http") {
            navitia_model::gtfs::read_from_url(gtfs, None::<&str>, None).unwrap()
        } else {
            navitia_model::gtfs::read_from_zip(gtfs, None::<&str>, None).unwrap()
        };
        log::info!("gtfs red");
        Self::new(nav_data, gtfs, &generation_period)
    }
}
