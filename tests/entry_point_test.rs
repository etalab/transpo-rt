use actix_web::http;
use actix_web::HttpMessage;
mod utils;

#[test]
fn entry_point_integration_test() {
    let _log_guard = utils::init_log();
    let mut srv = utils::make_simple_test_server();

    test_entrypoint(&mut srv);
    test_dataset_entrypoint(&mut srv);
}

fn test_entrypoint(srv: &mut actix_web::test::TestServer) {
    let request = srv.client(http::Method::GET, "/").finish().unwrap();
    let response = srv.execute(request.send()).unwrap();

    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let resp: serde_json::Value = serde_json::from_str(body).unwrap();
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
                        "href": &srv.url("/spec")
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
                        "name": "default name"
                    }
                ]
            }
        }
    );
}

fn test_dataset_entrypoint(srv: &mut actix_web::test::TestServer) {
    let request = srv.client(http::Method::GET, "/default/").finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let bytes = srv.execute(response.body()).unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();

    let mut resp: serde_json::Value = serde_json::from_str(body).unwrap();

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
                "_links": {
                    "general-message": {
                        "href": &srv.url("/default/siri/2.0/general-message.json")
                    },
                    "gtfs-rt": {
                        "href": &srv.url("/default/gtfs-rt")
                    },
                    "gtfs-rt.json": {
                        "href": &srv.url("/default/gtfs-rt.json")
                    },
                    "stop-monitoring": {
                        "href": &srv.url("/default/siri/2.0/stop-monitoring.json")
                    },
                    "stoppoints-discovery": {
                        "href": &srv.url("/default/siri/2.0/stoppoints-discovery.json")
                    }
                }
            }
        }
    );
}
