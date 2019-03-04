pub mod service_delivery;
pub mod shared;
pub mod stop_points_delivery;

use service_delivery::ServiceDelivery;
use stop_points_delivery::StopPointsDelivery;

pub use shared::DateTime;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Siri {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_points_delivery: Option<StopPointsDelivery>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_delivery: Option<ServiceDelivery>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub general_message_delivery: Option<GeneralMessageDelivery>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SiriResponse {
    pub siri: Siri,
}
