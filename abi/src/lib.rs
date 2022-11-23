mod pb;
use chrono::{DateTime, NaiveDateTime, Utc};
pub use pb::*;
use prost_types::Timestamp;

pub fn convert_to_utc_time(ts: Timestamp) -> DateTime<Utc> {
    let naive = NaiveDateTime::from_timestamp_opt(ts.seconds, ts.nanos as u32).unwrap();
    DateTime::<Utc>::from_utc(naive, Utc)
}
