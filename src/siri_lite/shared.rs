#[derive(Debug)]
pub struct DateTime(pub chrono::NaiveDateTime);

impl std::string::ToString for DateTime {
    fn to_string(&self) -> String {
        self.0.format("%Y-%m-%dT%H:%M:%S").to_string()
    }
}

impl serde::Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> ::serde::Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(DateTime(
            chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").map_err(|e| {
                serde::de::Error::custom(format!("datetime format not valid: {}", e))
            })?,
        ))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorCondition {}

/// Common fields used by all the siri's Delivery
///
/// Note: it is referenced as `xxxDelivery` in the siri specifications
#[derive(Serialize, Deserialize, Debug)]
pub struct CommonDelivery {
    pub version: String,
    pub response_time_stamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Id of the query
    pub request_message_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_condition: Option<ErrorCondition>,
}

impl Default for CommonDelivery {
    fn default() -> Self {
        CommonDelivery {
            version: "2.0".to_string(),
            response_time_stamp: chrono::Utc::now().to_rfc3339(),
            error_condition: None,
            status: Some(true),
            request_message_ref: None,
        }
    }
}
