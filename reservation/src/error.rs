use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReservationError {
    #[error("database error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("Reservation error")]
    Unknown,
    #[error("Reservation start time or end time is invalid")]
    InvalidTime,
}
