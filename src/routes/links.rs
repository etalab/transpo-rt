use openapi_schema::OpenapiSchema;
use std::collections::BTreeMap;

/// Links as described in "JSON Hypertext Application Language"
/// https://tools.ietf.org/html/draft-kelly-json-hal-08
#[derive(Serialize, Deserialize, Debug, Default, Clone, OpenapiSchema)]
pub struct Links {
    #[serde(rename = "_links", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, Link>,
}

impl From<BTreeMap<&str, Link>> for Links {
    fn from(hash: BTreeMap<&str, Link>) -> Self {
        Self {
            links: hash.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, OpenapiSchema)]
pub struct Link {
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templated: Option<bool>,
}

impl Link {
    pub fn from_url(req: &actix_web::HttpRequest, name: &str, params: &[&str]) -> Self {
        Self {
            href: req.url_for(name, params).map(|u| u.into_string()).unwrap(),
            ..Default::default()
        }
    }
}
