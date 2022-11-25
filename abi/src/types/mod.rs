use std::ops::Bound;

use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use sqlx::postgres::types::PgRange;

use crate::{convert_to_utc_time, Error};

mod reservation;
mod reservation_query;
mod reservation_status;

pub fn validate_range(start: Option<&Timestamp>, end: Option<&Timestamp>) -> Result<(), Error> {
    let f = |op: Option<&Timestamp>| match op {
        Some(ts) => Ok(ts.seconds),
        None => Err(Error::InvalidTime),
    };
    let start = f(start).unwrap();
    let end = f(end).unwrap();
    if start >= end {
        return Err(Error::InvalidTime);
    }
    Ok(())
}

pub fn get_timespan(start: Option<&Timestamp>, end: Option<&Timestamp>) -> PgRange<DateTime<Utc>> {
    let f = |op: Option<&Timestamp>| match op {
        Some(ts) => Ok(convert_to_utc_time(ts.clone())),
        None => Err(Error::InvalidTime),
    };
    let start = f(start).unwrap();
    let end = f(end).unwrap();
    PgRange {
        start: Bound::Included(start),
        end: Bound::Excluded(end),
    }
}
