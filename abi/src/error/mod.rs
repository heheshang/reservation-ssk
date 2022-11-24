use sqlx::postgres::PgDatabaseError;
use thiserror::Error;
mod conflict;
pub use conflict::{ReservationConflict, ReservationConflictInfo, ReservationWindow};

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx error: {0}")]
    DbError(sqlx::Error),
    #[error("Invalid user id {0}")]
    InvalidUserId(String),

    #[error("Invalid resource id {0}")]
    InvalidResourceId(String),

    #[error("Invalid reservation id {0}")]
    InvalidReservationId(String),

    #[error("conflict reservation: {0:?}")]
    ReservationConflict(ReservationConflictInfo),

    // #[error("database error: {0}")]
    // DbError(#[from] sqlx::Error),
    #[error("Reservation error")]
    Unknown,
    #[error("Reservation start time or end time is invalid")]
    InvalidTime,
    #[error("No reservation found by the given condition {0}")]
    NotFound(String),
}
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // todo: check if this is correct
            (Self::DbError(_), Self::DbError(_)) => true,
            (Self::InvalidUserId(v1), Self::InvalidUserId(v2)) => v1 == v2,
            (Self::InvalidResourceId(v1), Self::InvalidResourceId(v2)) => v1 == v2,
            (Self::InvalidReservationId(v1), Self::InvalidReservationId(v2)) => v1 == v2,
            (Self::ReservationConflict(v1), Self::ReservationConflict(v2)) => v1 == v2,
            (Self::Unknown, Self::Unknown) => true,
            (Self::InvalidTime, Self::InvalidTime) => true,
            (Self::NotFound(_), Self::NotFound(_)) => true,
            _ => false,
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::Database(e) => {
                let err: &PgDatabaseError = e.downcast_ref();
                match (err.code(), err.schema(), err.table()) {
                    ("23P01", Some("rsvp"), Some("reservations")) => {
                        Error::ReservationConflict(err.detail().unwrap().parse().unwrap())
                    }

                    _ => Error::DbError(sqlx::Error::Database(e)),
                }
            }
            sqlx::Error::RowNotFound => Error::NotFound("row not found".to_string()),
            _ => Error::DbError(e),
        }
    }
}
