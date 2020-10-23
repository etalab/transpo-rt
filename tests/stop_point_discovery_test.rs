use std::collections::BTreeSet;
use transpo_rt::siri_lite::SiriResponse;

mod utils;

#[actix_rt::test]
async fn sp_discovery_integration_test() {
    let _log_guard = utils::init_log();
    let mut srv = utils::make_simple_test_server().await;

    filter_query(&mut srv).await;
    limit_query(&mut srv).await;
}

async fn filter_query(srv: &mut actix_web::test::TestServer) {
    let resp: SiriResponse =
        utils::get_json(srv, "/default/siri/2.0/stoppoints-discovery.json?q=mai").await;
    let spd = resp.siri.stop_points_delivery.unwrap();
    assert_eq!(spd.common.version, "2.0");
    assert_eq!(spd.common.status, Some(true));
    // no filtering, we fetch all stops
    assert_eq!(spd.annotated_stop_point.len(), 1);

    let stop1 = spd
        .annotated_stop_point
        .iter()
        .find(|s| s.stop_point_ref == "EMSI")
        .unwrap();

    assert_eq!(stop1.stop_name, "E Main St / S Irving St (Demo)");
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(stop1.location.longitude, -116.762_18_f64);
        assert_eq!(stop1.location.latitude, 36.905_697_f64);
    }
    assert_eq!(
        stop1
            .lines
            .iter()
            .map(|l| l.line_ref.clone())
            .collect::<BTreeSet<_>>(),
        vec!["CITY_R".into(), "CITY".into()].into_iter().collect()
    );
    //TODO more tests
}

async fn limit_query(srv: &mut actix_web::test::TestServer) {
    let resp: SiriResponse =
        utils::get_json(srv, "/default/siri/2.0/stoppoints-discovery.json?").await;

    // with no filtering there are 9 stops
    let spd = resp.siri.stop_points_delivery.unwrap();
    assert_eq!(spd.annotated_stop_point.len(), 9);
    let resp: SiriResponse =
        utils::get_json(srv, "/default/siri/2.0/stoppoints-discovery.json?limit=3").await;

    // if we ask for only 3 stops, we got only 3
    let spd = resp.siri.stop_points_delivery.unwrap();
    assert_eq!(spd.annotated_stop_point.len(), 3);
}
