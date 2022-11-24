use sqlx::postgres::PgDatabaseError;
use thiserror::Error;
mod conflict;
pub use conflict::{ReservationConflictInfo, ReservationWindow};

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx error: {0}")]
    DbError(sqlx::Error),
    #[error("Invalid user id {0}")]
    InvalidUserId(String),

    #[error("Invalid resource id {0}")]
    InvalidResourceId(String),

    #[error("conflict reservation: {0:?}")]
    ConflictReservation(ReservationConflictInfo),

    // #[error("database error: {0}")]
    // DbError(#[from] sqlx::Error),
    #[error("Reservation error")]
    Unknown,
    #[error("Reservation start time or end time is invalid")]
    InvalidTime,
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::Database(e) => {
                let err: &PgDatabaseError = e.downcast_ref();
                match (err.code(), err.schema(), err.table()) {
                    ("23P01", Some("rsvp"), Some("reservations")) => {
                        Error::ConflictReservation(err.detail().unwrap().parse().unwrap())
                    }

                    _ => Error::DbError(sqlx::Error::Database(e)),
                }
            }
            _ => Error::DbError(e),
        }
    }
}
