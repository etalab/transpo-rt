#[macro_export]
macro_rules! skip_fail {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                log::warn!("{}", e);
                continue;
            }
        }
    };
}

use openapi::v3_0::{ObjectOrReference, Schema, Spec};
use openapi_schema::OpenapiSchema;

/// Duration that deseialize to ISO 8601
#[derive(Debug)]
pub struct Duration(chrono::Duration);

impl std::ops::Deref for Duration {
    type Target = chrono::Duration;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> ::serde::Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let dur = time_parse::duration::parse(&s)
            .ok()
            .and_then(|d| chrono::Duration::from_std(d).ok())
            .ok_or_else(|| serde::de::Error::custom("invalid duration".to_owned()))?;
        Ok(Duration(dur))
    }
}

impl OpenapiSchema for Duration {
    fn generate_schema(_spec: &mut Spec) -> ObjectOrReference<Schema> {
        ObjectOrReference::Object(Schema {
            schema_type: Some("string".into()),
            format: Some("duration".into()),
            ..Default::default()
        })
    }
}

pub fn read_pbf_dt(dt: Option<u64>, timezone: chrono_tz::Tz) -> Option<chrono::NaiveDateTime> {
    dt.map(|t| {
        chrono::DateTime::<chrono::Utc>::from_utc(
            chrono::NaiveDateTime::from_timestamp(t as i64, 0),
            chrono::Utc,
        )
    })
    .map(|utc_dt| utc_dt.with_timezone(&timezone))
    .map(|local_dt| local_dt.naive_local())
}

pub fn init_logger() -> slog_scope::GlobalLoggerGuard {
    use slog::Drain;

    let drain = slog_term::FullFormat::new(slog_term::TermDecorator::new().stderr().build())
        .build()
        .fuse();

    let builder = slog_envlogger::LogBuilder::new(drain).filter(None, slog::FilterLevel::Info);
    let builder = match std::env::var("RUST_LOG") {
        Ok(s) => builder.parse(&s),
        _ => builder,
    };
    let drain = slog_async::Async::new(builder.build())
        .chan_size(512)
        .build();

    let log = slog::Logger::root(drain.fuse(), slog::slog_o!());
    let scope_guard = slog_scope::set_global_logger(log);
    slog_stdlog::init().unwrap();
    scope_guard
}
