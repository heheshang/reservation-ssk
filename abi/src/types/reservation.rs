use std::ops::Bound;

use crate::{
    error::Error, get_timespan, utils::convert_to_timestamp, validate_range, Reservation,
    ReservationStatus, RsvpStatus, Validator,
};
use chrono::{DateTime, FixedOffset, Utc};
use sqlx::{
    postgres::{types::PgRange, PgRow},
    FromRow, Row,
};

impl Reservation {
    pub fn new_pending(
        uid: impl Into<String>,
        rid: impl Into<String>,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            id: 0,
            resource_id: rid.into(),
            user_id: uid.into(),
            start: Some(convert_to_timestamp(start.with_timezone(&Utc))),
            end: Some(convert_to_timestamp(end.with_timezone(&Utc))),
            note: note.into(),
            status: ReservationStatus::Pending as i32,
        }
    }

    pub fn get_time_span(&self) -> PgRange<DateTime<Utc>> {
        get_timespan(self.start.as_ref(), self.end.as_ref())
    }
}

impl Validator for Reservation {
    fn validate(&self) -> Result<(), Error> {
        validate_range(self.start.as_ref(), self.end.as_ref())
    }
}

impl FromRow<'_, PgRow> for Reservation {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let id: i64 = row.get("id");
        let range: PgRange<DateTime<Utc>> = row.get("timespan");
        let range: NaiveRange<DateTime<Utc>> = range.into();

        assert!(range.start.is_some());
        assert!(range.end.is_some());

        let start = range.start.unwrap();
        let end = range.end.unwrap();

        let status: RsvpStatus = row.get("status");

        Ok(Self {
            id,
            resource_id: row.get("resource_id"),
            user_id: row.get("user_id"),
            start: Some(convert_to_timestamp(start)),
            end: Some(convert_to_timestamp(end)),
            note: row.get("note"),
            status: ReservationStatus::from(status) as i32,
        })
    }
}

pub struct NaiveRange<T> {
    start: Option<T>,
    end: Option<T>,
}

impl<T> From<PgRange<T>> for NaiveRange<T> {
    fn from(range: PgRange<T>) -> Self {
        let f = |b: Bound<T>| match b {
            Bound::Included(v) => Some(v),
            Bound::Excluded(v) => Some(v),
            Bound::Unbounded => None,
        };

        let start = f(range.start);
        let end = f(range.end);

        Self { start, end }
    }
}
