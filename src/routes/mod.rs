mod general_message;
mod gtfs_rt;
mod list_datasets;
pub(crate) mod open_api;
mod status;
mod stop_monitoring;
mod stoppoints_discovery;

pub use self::general_message::general_message_query;
pub use self::gtfs_rt::{gtfs_rt, gtfs_rt_json};
pub use self::list_datasets::list_datasets;
pub use self::open_api::documentation;
pub use self::status::status_query;
pub use self::stop_monitoring::stop_monitoring_query;
pub use self::stoppoints_discovery::sp_discovery;

// export the params/responses for the openapi module
pub(crate) use self::general_message::Params as GeneralMessageParams;
pub(crate) use self::status::Status;
pub(crate) use self::stop_monitoring::Params as StopMonitoringParams;
pub(crate) use self::stoppoints_discovery::Params as StopPointsDiscoveryParams;
