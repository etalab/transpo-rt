use chrono::NaiveDate;

pub fn make_test_server() -> actix_web::test::TestServer {
    let begin = NaiveDate::from_ymd(2018, 12, 15);
    let period = transpo_rt::context::Period {
        begin: begin.clone(),
        end: begin.succ(),
    };
    let ctx = transpo_rt::server::make_context("fixtures/gtfs.zip", "", &period);
    let make_server = move || transpo_rt::server::create_server(ctx.clone());

    actix_web::test::TestServer::with_factory(make_server)
}
