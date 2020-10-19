use actix_web::{web, get};
use maplit::btreemap;
use openapi::v3_0 as oa;
use openapi_schema::OpenapiSchema;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn make_param<T: OpenapiSchema>(
    spec: &mut openapi::v3_0::Spec,
    name: &str,
    required: bool,
) -> openapi::v3_0::Parameter {
    use openapi::v3_0::{ObjectOrReference, Parameter, Schema};
    Parameter {
        name: name.to_owned(),
        location: "query".to_owned(),
        required: Some(required),
        schema: Some(match T::generate_schema(spec) {
            ObjectOrReference::Object(schema) => schema,
            ObjectOrReference::Ref { ref_path } => Schema {
                ref_path: Some(ref_path),
                ..Default::default()
            },
        }),
        ..Default::default()
    }
}

fn add_dataset_param(
    spec: &mut oa::Spec,
    params: &mut Vec<oa::ObjectOrReference<oa::Parameter>>,
    route: &str,
) {
    // ugly patch to add the dataset path parameters
    if route.starts_with("/{dataset}") {
        params.push(oa::ObjectOrReference::Object(oa::Parameter {
            name: "dataset".to_owned(),
            location: "path".to_owned(),
            required: Some(true),
            schema: Some(match String::generate_schema(spec) {
                oa::ObjectOrReference::Object(schema) => schema,
                _ => unreachable!(),
            }),
            ..Default::default()
        }));
    }
}

macro_rules! add_route {
    ($spec:expr, $route:expr => $response_type:ty, description = $description: expr,
    params = $params:expr, array = $array:expr) => {
        let type_name = stringify!($response_type);
        let type_name = type_name.split("::").last().unwrap();
        let path = format!("#/components/schemas/{}", type_name);

        let mut params: Vec<_> = $params.into_iter().map(|p| {
            oa::ObjectOrReference::Object(p)
        }).collect();
        add_dataset_param(&mut $spec, &mut params, $route);

        add_path_item::<$response_type>(&mut $spec, $route, $description, path, params, $array);
    };
    ($spec:expr, $route:expr => $response_type:ty, description = $description: expr, params = $params:expr) => {
        add_route!($spec, $route => $response_type, description = $description, params = $params, array = false)
    }
}

fn add_path_item<T: OpenapiSchema>(
    mut spec: &mut oa::Spec,
    route: &str,
    description: &str,
    path: String,
    params: Vec<oa::ObjectOrReference<oa::Parameter>>,
    is_array: bool,
) {
    let response_spec = if is_array {
        oa::MediaType {
            schema: Some(oa::ObjectOrReference::Object(oa::Schema {
                schema_type: Some("array".to_owned()),
                items: Some(Box::new(oa::Schema {
                    ref_path: Some(path),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        }
    } else {
        oa::MediaType {
            schema: Some(oa::ObjectOrReference::Ref { ref_path: path }),
            ..Default::default()
        }
    };
    let params = if params.is_empty() {
        None
    } else {
        Some(params)
    };
    spec.paths.insert(
        route.to_owned(),
        oa::PathItem {
            get: Some(oa::Operation {
                responses: btreemap! {
                    "200".to_string() => oa::Response {
                        description: Some(description.to_owned()),
                        content: Some(btreemap!{
                            "application/json".to_owned() => response_spec
                        }),
                        ..Default::default()
                    }
                },
                parameters: params,
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    T::generate_schema(&mut spec);
}

fn add_path_item_with_undefined_response(
    spec: &mut oa::Spec,
    route: &str,
    description: &str,
    content_type: &str,
) {
    let mut params = vec![];
    add_dataset_param(spec, &mut params, route);
    let params = if params.is_empty() {
        None
    } else {
        Some(params)
    };
    let response_spec = oa::MediaType::default();
    spec.paths.insert(
        route.to_owned(),
        oa::PathItem {
            get: Some(oa::Operation {
                responses: btreemap! {
                    "200".to_string() => oa::Response {
                        description: Some(description.to_owned()),
                        content: Some(btreemap!{
                            content_type.to_owned() => response_spec
                        }),
                        ..Default::default()
                    }
                },
                parameters: params,
                ..Default::default()
            }),
            ..Default::default()
        },
    );
}

fn create_schema() -> oa::Spec {
    let mut spec = oa::Spec::default();
    spec.openapi = "3.0.0".to_owned();
    spec.info.title = "Transpo-rt".to_owned();
    spec.info.version = VERSION.to_owned();
    add_route!(spec, "/" => crate::datasets::DatasetInfo, description = "list all the datasets", params = vec![], array = true);
    add_path_item_with_undefined_response(
        &mut spec,
        "/spec",
        "openapi documentation",
        "application/json",
    );

    add_route!(spec, "/{dataset}" => super::Status, description = "status of a dataset", params = vec![]);
    add_route!(spec, "/{dataset}/siri/2.0/stop-monitoring.json" => crate::siri_lite::SiriResponse,
                description = "siri-lite stop monitoring",
                params = super::StopMonitoringParams::openapi_description(&mut spec));
    add_route!(spec, "/{dataset}/siri/2.0/stoppoints-discovery.json" => crate::siri_lite::SiriResponse,
                description = "siri-lite stop discovery",
                params = super::StopPointsDiscoveryParams::openapi_description(&mut spec));
    add_route!(spec, "/{dataset}/siri/2.0/general-message.json" => crate::siri_lite::SiriResponse,
                description = "siri-lite general message",
                params = super::GeneralMessageParams::openapi_description(&mut spec));

    // for gtfs-rt we don't really want to define the response, it's too complex
    add_path_item_with_undefined_response(
        &mut spec,
        "/{dataset}/gtfs-rt.json",
        "json of the gtfs-rt",
        "application/json",
    );
    add_path_item_with_undefined_response(
        &mut spec,
        "/{dataset}/gtfs-rt",
        "raw gtfs-rt (protobuf)",
        "application/x-protobuf",
    );
    spec
}

#[get("/documentation")]
pub async fn documentation() -> web::Json<openapi::v3_0::Spec> {
    let mut spec = create_schema();

    crate::siri_lite::SiriResponse::generate_schema(&mut spec);
    web::Json(spec)
}
