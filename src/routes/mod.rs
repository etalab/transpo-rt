mod api_entry_point;
mod exposed_dataset;
// mod general_message;
mod gtfs_rt;
mod links;
// pub(crate) mod open_api;
// mod siri;
mod status;
// mod stop_monitoring;
// mod stoppoints_discovery;

pub use self::api_entry_point::entry_point;
pub use self::exposed_dataset::ExposedDataset;
// pub use self::general_message::general_message_query;
pub use self::gtfs_rt::{gtfs_rt, gtfs_rt_json};
pub use self::links::{Link, Links};
// pub use self::open_api::documentation;
// pub use self::siri::siri_endpoint;
pub use self::status::status_query;
// pub use self::stop_monitoring::stop_monitoring_query;
// pub use self::stoppoints_discovery::sp_discovery;

// export the params/responses for the openapi module
// pub(crate) use self::general_message::Params as GeneralMessageParams;
// pub(crate) use self::status::Status;
// pub(crate) use self::stop_monitoring::Params as StopMonitoringParams;
// pub(crate) use self::stoppoints_discovery::Params as StopPointsDiscoveryParams;
