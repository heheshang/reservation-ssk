mod config;
mod request;
mod reservation;
mod reservation_filter;
mod reservation_query;
mod reservation_status;

use std::ops::Bound;

use crate::{convert_to_utc_time, Error};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use sqlx::postgres::types::PgRange;

pub use config::*;

pub fn validate_range(start: Option<&Timestamp>, end: Option<&Timestamp>) -> Result<(), Error> {
    if start.is_none() || end.is_none() {
        return Err(Error::InvalidTime);
    }

    let start = start.as_ref().unwrap();
    let end = end.as_ref().unwrap();
    if start.seconds >= end.seconds {
        return Err(Error::InvalidTime);
    }
    Ok(())
}

pub fn get_timespan(start: Option<&Timestamp>, end: Option<&Timestamp>) -> PgRange<DateTime<Utc>> {
    let f = |op: Option<&Timestamp>| match op {
        Some(ts) => Ok(convert_to_utc_time(ts)),
        None => Err(Error::InvalidTime),
    };
    let start = f(start).unwrap();
    let end = f(end).unwrap();
    PgRange {
        start: Bound::Included(start),
        end: Bound::Excluded(end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost_types::Timestamp;

    #[test]
    fn test_i64_default() {
        let ss: i64 = Default::default();
        println!("{}", ss);
        assert_eq!(ss, 0);
    }
    #[test]
    fn validate_range_should_work() {
        let start = Some(&Timestamp {
            seconds: 1,
            nanos: 0,
        });
        let end = Some(&Timestamp {
            seconds: 2,
            nanos: 0,
        });
        assert!(validate_range(start, end).is_ok());
    }
    #[test]
    fn validate_range_should_fail() {
        let start = Some(&Timestamp {
            seconds: 2,
            nanos: 0,
        });
        let end = Some(&Timestamp {
            seconds: 1,
            nanos: 0,
        });
        assert!(validate_range(start, end).is_err());
    }
    #[test]
    fn get_timespan_should_work() {
        let start = &Timestamp {
            seconds: 1,
            nanos: 0,
        };
        let end = &Timestamp {
            seconds: 2,
            nanos: 0,
        };
        let timespan = get_timespan(Some(start), Some(end));
        assert_eq!(timespan.start, Bound::Included(convert_to_utc_time(start)));
        assert_eq!(timespan.end, Bound::Excluded(convert_to_utc_time(end)));
    }
}
