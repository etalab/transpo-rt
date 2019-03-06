#[macro_export]
macro_rules! skip_fail {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                warn!("{}", e);
                continue;
            }
        }
    };
}

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
