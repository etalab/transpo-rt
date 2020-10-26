use anyhow::anyhow;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use log::info;
use openapi_schema::OpenapiSchema;
use std::collections::HashMap;
use std::sync::Arc;
use transit_model::collection::Idx;

use crate::transit_realtime;

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
    pub vj_idx: Idx<transit_model::objects::VehicleJourney>,
    pub date: chrono::NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    pub dated_vj: DatedVehicleJourney,
    pub stop_point_idx: Idx<transit_model::objects::StopPoint>,
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
    pub dataset_info: DatasetInfo,
    pub generation_period: Period,
}

pub struct Dataset {
    pub ntm: transit_model::Model,
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

#[derive(Deserialize, Clone, Default)]
pub struct Datasets {
    pub datasets: Vec<DatasetInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, OpenapiSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub struct DatasetInfo {
    pub name: String,
    pub id: String,
    pub gtfs: String,
    pub gtfs_rt_urls: Vec<String>,
    #[serde(default)]
    pub extras: std::collections::BTreeMap<String, String>,
}

impl DatasetInfo {
    pub fn new_default(gtfs: &str, gtfs_rt_urls: &[String]) -> Self {
        Self {
            id: "default".into(),
            name: "default name".into(),
            gtfs: gtfs.to_owned(),
            gtfs_rt_urls: gtfs_rt_urls.to_vec(),
            extras: std::collections::BTreeMap::default(),
        }
    }
}

// create a dt from a Date and a StopTime's time
// Note: the time might be on the next day, for example "26:00:00"
// is the next day at 2 in the morning
fn create_dt(date: NaiveDate, time: transit_model::objects::Time) -> NaiveDateTime {
    date.and_time(chrono::NaiveTime::from_hms(0, 0, 0))
        + chrono::Duration::seconds(i64::from(time.total_seconds()))
}

fn create_timetable(ntm: &transit_model::Model, generation_period: &Period) -> Timetable {
    info!("computing timetable for {:?}", &generation_period);
    let begin_dt = Utc::now();
    let mut timetable = Timetable {
        connections: vec![],
    };
    let begin = generation_period.begin;
    let end = begin + generation_period.horizon;

    for (vj_idx, vj) in ntm.vehicle_journeys.iter() {
        let service = skip_fail!(ntm.calendars.get(&vj.service_id).ok_or_else(|| anyhow!(
            "impossible to find service {} for vj {}, skipping vj",
            &vj.service_id,
            &vj.id
        )));
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
        use prost::Message;
        transit_realtime::FeedMessage::decode(self.data.as_slice())
            .map_err(|e| log::warn!("Unable to decode feed message, {}", e))
            .ok()
    }
}

pub trait HasTimezone {
    fn timezone(&self) -> Option<chrono_tz::Tz>;
}

impl HasTimezone for transit_model::Model {
    fn timezone(&self) -> Option<chrono_tz::Tz> {
        self.networks
            .values()
            .next()
            .and_then(|n| n.timezone.as_ref())
            .and_then(|t| {
                t.parse()
                    .map_err(|e| log::warn!("impossible to parse timezone {} because: {}", t, e))
                    .ok()
            })
    }
}

impl Dataset {
    fn new(
        dataset_info: DatasetInfo,
        ntm: transit_model::Model,
        generation_period: &Period,
    ) -> Self {
        // To correctly handle GTFS-RT stream we need the dataset's timezone,
        // as all the time in the dataset are in local time and the GTFS-RT gives its time
        // as UTC.
        // We consider that there can be at most one Company (gtfs's agency) in the dataset
        // and we consider that the dataset's timezone is it's agency's timezone
        let timezone = ntm
            .timezone()
            .expect("no timezone found, we will not be able to understand realtime information");

        Self {
            timetable: create_timetable(&ntm, generation_period),
            ntm,
            timezone,
            loaded_at: chrono::Utc::now(),
            feed_construction_info: FeedConstructionInfo {
                dataset_info,
                generation_period: generation_period.clone(),
            },
        }
    }

    pub fn try_from_dataset_info(
        dataset_info: DatasetInfo,
        generation_period: &Period,
    ) -> Result<Self, anyhow::Error> {
        log::info!("reading from path");
        let gtfs = dataset_info.gtfs.as_str();
        let nav_data = if gtfs.starts_with("http") {
            transit_model::gtfs::read_from_url(gtfs, None::<&str>, None)
        } else {
            transit_model::gtfs::read_from_zip(gtfs, None::<&str>, None)
        }
        .map_err(|e| anyhow!("impossible to read GTFS {} because {}", gtfs, e))?;
        log::info!("gtfs read");
        Ok(Self::new(dataset_info, nav_data, &generation_period))
    }
}

#[cfg(test)]
mod tests {
    use crate::datasets::{Connection, DatedVehicleJourney, Period};
    use transit_model_builder::ModelBuilder;

    #[test]
    fn test_timetable_creation() {
        let model = ModelBuilder::default()
            .calendar("c", |c| {
                c.dates.insert(chrono::NaiveDate::from_ymd(2019, 2, 6));
            })
            .vj("vj1", |vj_builder| {
                vj_builder
                    .calendar("c")
                    .st("A", "10:00:00", "10:01:00")
                    .st("B", "11:00:00", "11:01:00")
                    .st("C", "12:00:00", "12:01:00");
            })
            .vj("vj2", |vj_builder| {
                vj_builder
                    .calendar("c")
                    .st("B", "11:30:00", "11:31:00")
                    .st("D", "15:00:00", "15:01:00");
            })
            .build();

        let date = chrono::NaiveDate::from_ymd(2019, 2, 6);
        let period = Period {
            begin: date,
            horizon: chrono::Duration::days(1),
        };
        let timetable = super::create_timetable(&model, &period);
        assert_eq!(timetable.connections.len(), 5);

        assert_eq!(
            &timetable.connections[0],
            &Connection {
                dated_vj: DatedVehicleJourney {
                    vj_idx: model.vehicle_journeys.get_idx("vj1").unwrap(),
                    date: date,
                },
                stop_point_idx: model.stop_points.get_idx("A").unwrap(),
                dep_time: date.and_hms(10, 1, 0),
                arr_time: date.and_hms(10, 0, 0),
                sequence: 0,
            }
        );

        assert_eq!(
            &timetable.connections[1],
            &Connection {
                dated_vj: DatedVehicleJourney {
                    vj_idx: model.vehicle_journeys.get_idx("vj1").unwrap(),
                    date: date,
                },
                stop_point_idx: model.stop_points.get_idx("B").unwrap(),
                dep_time: date.and_hms(11, 1, 0),
                arr_time: date.and_hms(11, 0, 0),
                sequence: 1,
            }
        );

        assert_eq!(
            &timetable.connections[2],
            &Connection {
                dated_vj: DatedVehicleJourney {
                    vj_idx: model.vehicle_journeys.get_idx("vj2").unwrap(),
                    date: date,
                },
                stop_point_idx: model.stop_points.get_idx("B").unwrap(),
                dep_time: date.and_hms(11, 31, 0),
                arr_time: date.and_hms(11, 30, 0),
                sequence: 0,
            }
        );

        assert_eq!(
            &timetable.connections[3],
            &Connection {
                dated_vj: DatedVehicleJourney {
                    vj_idx: model.vehicle_journeys.get_idx("vj1").unwrap(),
                    date: date,
                },
                stop_point_idx: model.stop_points.get_idx("C").unwrap(),
                dep_time: date.and_hms(12, 1, 0),
                arr_time: date.and_hms(12, 0, 0),
                sequence: 2,
            }
        );

        assert_eq!(
            &timetable.connections[4],
            &Connection {
                dated_vj: DatedVehicleJourney {
                    vj_idx: model.vehicle_journeys.get_idx("vj2").unwrap(),
                    date: date,
                },
                stop_point_idx: model.stop_points.get_idx("D").unwrap(),
                dep_time: date.and_hms(15, 1, 0),
                arr_time: date.and_hms(15, 0, 0),
                sequence: 1,
            }
        );
    }
}
