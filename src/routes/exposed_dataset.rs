use crate::datasets::DatasetInfo;
use crate::routes::{Link, Links};
use openapi_schema::OpenapiSchema;

#[derive(Serialize, Debug, OpenapiSchema)]
pub struct ExposedDataset {
    pub name: String,
    pub id: String,
    pub gtfs: String,
    // we do not expose the gtfs-rt sources since the information can contains api key
    // so we add links to the gtfs-rt routes instead
    #[serde(flatten)]
    pub links: Links,
}

impl From<&DatasetInfo> for ExposedDataset {
    fn from(d: &DatasetInfo) -> Self {
        Self {
            name: d.name.clone(),
            id: d.id.clone(),
            gtfs: d.gtfs.clone(),
            links: Links::default(),
        }
    }
}

impl ExposedDataset {
    pub fn add_link(mut self, key: &str, href: &str) -> Self {
        self.links.links.insert(
            key.to_owned(),
            Link {
                href: href.to_owned(),
                ..Default::default()
            },
        );
        self
    }
}
