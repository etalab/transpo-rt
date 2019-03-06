use crate::siri_lite::DateTime;

// Note: this list seems to be specific to the idf profile
// it can be extended if needed
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(non_camel_case_types)]
pub enum MessageType {
    shortMessage,
    longMessage,
    textOnly,
    HTML,
    RTF,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NaturalLangString {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    // type of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_type: Option<MessageType>,
    /// body of the message
    pub message_text: NaturalLangString,
}

// Note: this seems to be a structure only for the idf profile
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct GeneralMessageStructure {
    /// Id of the impacted lines
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub line_ref: Vec<String>,
    /// Id of the impacted stop points
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop_point_ref: Vec<String>,
    /// Id of the impacted destinations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub destination_ref: Vec<String>,
    /// Messages
    pub message: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct InfoMessage {
    /// reference of the format used in the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// datetime of the recording of the message
    /// Note: this field is mandatory for the idf profile, but we cannot easily fill it with gtfs-rt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_at_time: Option<DateTime>,
    /// Uniq identifier of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_identifier: Option<String>,
    /// Uniq identifier of this information
    /// If this message needs to be updated, this info_message_identifier
    /// will be used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_message_identifier: Option<String>,
    /// version of this info message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_message_version: Option<String>,
    /// Datetime until this message is valid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until_time: Option<DateTime>,
    /// Content of the message
    pub content: GeneralMessageStructure,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InfoMessageCancellation {
    pub recorded_at_time: DateTime,
    /// Uniq identifier of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_identifier: Option<String>,
    /// Uniq identifier of the information to cancel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_message_identifier: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct GeneralMessageDelivery {
    #[serde(flatten)]
    pub common: crate::siri_lite::shared::CommonDelivery,
    pub info_messages: Vec<InfoMessage>,
    pub info_messages_cancellation: Vec<InfoMessage>,
}
