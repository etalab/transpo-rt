mod utils;
use maplit::btreeset;
use pretty_assertions::assert_eq;
use serde_json::Value;
use transpo_rt::datasets::DatasetInfo;
use utils::get_json;

#[actix_rt::test]
async fn entry_point_integration_test() {
    let _log_guard = utils::init_log();
    let mut srv = utils::make_simple_test_server().await;

    test_entrypoint(&mut srv).await;
    test_dataset_entrypoint(&mut srv).await;
    test_siri_entrypoint(&mut srv).await;
}

async fn test_entrypoint(srv: &mut actix_web::test::TestServer) {
    let resp: Value = get_json(srv, "/").await;
    assert_eq!(
        resp,
        serde_json::json! {
                {
                "_links": {
                    "dataset_detail": {
                        "href": &srv.url("/{id}/"),
                        "templated": true
                    },
                    "documentation": {
                        "href": &srv.url("/spec/")
                    }
                },
                "datasets": [
                    {
                        "_links": {
                            "self": {
                                "href": &srv.url("/default/")
                            }
                        },
                        "gtfs": "fixtures/gtfs.zip",
                        "id": "default",
                        "name": "default name",
                        "extras": {},
                    }
                ]
            }
        }
    );
}

async fn test_dataset_entrypoint(srv: &mut actix_web::test::TestServer) {
    let mut resp: Value = get_json(srv, "/default/").await;

    // we change the loaded_at datetime to be able to easily compare the response
    *resp.pointer_mut("/loaded_at").unwrap() = "2019-06-20T10:00:00Z".into();
    assert_eq!(
        resp,
        serde_json::json! {
            {
                "name": "default name",
                "id": "default",
                "gtfs": "fixtures/gtfs.zip",
                "loaded_at": "2019-06-20T10:00:00Z",
                "extras": {},
                "_links": {
                    "general-message": {
                        "href": &srv.url("/default/siri/2.0/general-message.json/")
                    },
                    "gtfs-rt": {
                        "href": &srv.url("/default/gtfs-rt/")
                    },
                    "gtfs-rt.json": {
                        "href": &srv.url("/default/gtfs-rt.json/")
                    },
                    "stop-monitoring": {
                        "href": &srv.url("/default/siri/2.0/stop-monitoring.json/")
                    },
                    "siri-lite": {
                        "href": &srv.url("/default/siri/2.0/")
                    },
                    "stoppoints-discovery": {
                        "href": &srv.url("/default/siri/2.0/stoppoints-discovery.json/")
                    }
                }
            }
        }
    );
}

async fn test_siri_entrypoint(srv: &mut actix_web::test::TestServer) {
    let resp: Value = get_json(srv, "/default/siri/2.0/").await;

    assert_eq!(
        resp,
        serde_json::json! {
            {
                "_links": {
                    "general-message": {
                        "href": &srv.url("/default/siri/2.0/general-message.json/")
                    },
                    "stop-monitoring": {
                        "href": &srv.url("/default/siri/2.0/stop-monitoring.json/")
                    },
                    "stoppoints-discovery": {
                        "href": &srv.url("/default/siri/2.0/stoppoints-discovery.json/")
                    }
                }
            }
        }
    );
}

// test that invalid dataset are filtered
#[actix_rt::test]
async fn invalid_dataset_test() {
    let _log_guard = utils::init_log();
    let mut srv = utils::make_test_server(vec![
        DatasetInfo {
            id: "a_valid_dataset".into(),
            name: "valid dataset".into(),
            gtfs: "fixtures/gtfs.zip".to_owned(),
            gtfs_rt_urls: [mockito::server_url() + "/gtfs_rt_1"].to_vec(),
            extras: std::collections::BTreeMap::default(),
        },
        DatasetInfo {
            id: "a_non_valid_dataset".into(),
            name: "non valid dataset".into(),
            gtfs: "non_existing_gtfs.zip".to_owned(),
            gtfs_rt_urls: [mockito::server_url() + "/gtfs_rt_1"].to_vec(),
            extras: std::collections::BTreeMap::default(),
        },
    ])
    .await;

    let resp: Value = get_json(&mut srv, "/").await;

    // The 2 datasets should be loaded
    // for the moment the '/' route has no information on the status of the dataset
    assert_eq!(
        resp.get("datasets")
            .and_then(|v| v.as_array())
            .expect("should be an array")
            .iter()
            .map(|d| d.get("id").unwrap().as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>(),
        btreeset! {"a_valid_dataset", "a_non_valid_dataset"}
    );
}

#[actix_rt::test]
async fn multiple_datasets_test() {
    use transpo_rt::datasets::DatasetInfo;
    let _log_guard = utils::init_log();
    let first_dataset = DatasetInfo {
        id: "first_dataset".into(),
        name: "First dataset".into(),
        gtfs: "fixtures/gtfs.zip".to_owned(),
        gtfs_rt_urls: [mockito::server_url() + "/gtfs_rt_1"].to_vec(),
        extras: std::collections::BTreeMap::default(),
    };
    let second_dataset = DatasetInfo {
        id: "second_dataset".into(),
        name: "Seond dataset".into(),
        gtfs: "fixtures/gtfs.zip".to_owned(),
        gtfs_rt_urls: [mockito::server_url() + "/gtfs_rt_1"].to_vec(),
        extras: std::collections::BTreeMap::default(),
    };
    let mut srv =
        utils::make_test_server(vec![first_dataset.clone(), second_dataset.clone()]).await;

    check_datasets_entrypoints(&mut srv, &first_dataset).await;
    check_datasets_entrypoints(&mut srv, &second_dataset).await;

    // check that trailing slashes are handled
    // no particular check on the response is done, we just check that their status are 200
    get_json::<Value>(&mut srv, "/spec").await;
    get_json::<Value>(&mut srv, "/spec/").await;
    get_json::<Value>(&mut srv, "/first_dataset").await;
    get_json::<Value>(&mut srv, "/first_dataset/").await;
    get_json::<Value>(&mut srv, "/first_dataset/siri/2.0/").await;
    get_json::<Value>(&mut srv, "/first_dataset/siri/2.0").await;
    get_json::<Value>(
        &mut srv,
        "/first_dataset/siri/2.0/stoppoints-discovery.json/",
    )
    .await;
    get_json::<Value>(
        &mut srv,
        "/first_dataset/siri/2.0/stoppoints-discovery.json",
    )
    .await;
}

async fn check_datasets_entrypoints(srv: &mut actix_web::test::TestServer, dataset: &DatasetInfo) {
    let mut resp: Value = get_json(srv, &format!("/{}/", dataset.id)).await;

    // we change the loaded_at datetime to be able to easily compare the response
    *resp.pointer_mut("/loaded_at").unwrap() = "2019-06-20T10:00:00Z".into();
    assert_eq!(
        resp,
        serde_json::json! {
            {
                "name": &dataset.name,
                "id": &dataset.id,
                "gtfs": "fixtures/gtfs.zip",
                "loaded_at": "2019-06-20T10:00:00Z",
                "extras": {},
                "_links": {
                    "general-message": {
                        "href": &srv.url(&format!("/{}/siri/2.0/general-message.json/", &dataset.id))
                    },
                    "gtfs-rt": {
                        "href": &srv.url(&format!("/{}/gtfs-rt/", &dataset.id))
                    },
                    "gtfs-rt.json": {
                        "href": &srv.url(&format!("/{}/gtfs-rt.json/", &dataset.id))
                    },
                    "stop-monitoring": {
                        "href": &srv.url(&format!("/{}/siri/2.0/stop-monitoring.json/", &dataset.id))
                    },
                    "siri-lite": {
                        "href": &srv.url(&format!("/{}/siri/2.0/", &dataset.id))
                    },
                    "stoppoints-discovery": {
                        "href": &srv.url(&format!("/{}/siri/2.0/stoppoints-discovery.json/", &dataset.id))
                    }
                }
            }
        }
    );
}
