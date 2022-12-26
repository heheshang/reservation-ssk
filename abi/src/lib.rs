mod error;
mod pager;
mod pb;
mod types;
mod utils;

pub use error::*;
pub use pb::*;
pub use types::*;
pub use utils::*;

pub type ReservationId = i64;
pub type InvalidUserId = String;
pub type InvalidResourceId = String;

pub trait Validator {
    fn validate(&self) -> Result<(), Error>;
}

pub trait Normalizer: Validator {
    fn normalize(&mut self) -> Result<(), Error> {
        self.validate()?;
        self.do_normalize();
        Ok(())
    }
    fn do_normalize(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "reservation_status", rename_all = "lowercase")]
pub enum RsvpStatus {
    Pending,
    Blocked,
    Confirmed,
    Unknown,
}
impl From<RsvpStatus> for ReservationStatus {
    fn from(status: RsvpStatus) -> Self {
        match status {
            RsvpStatus::Pending => ReservationStatus::Pending,
            RsvpStatus::Blocked => ReservationStatus::Blocked,
            RsvpStatus::Confirmed => ReservationStatus::Confirmed,
            RsvpStatus::Unknown => ReservationStatus::Unknown,
        }
    }
}

impl Validator for ReservationId {
    fn validate(&self) -> Result<(), Error> {
        if *self <= 0 {
            return Err(Error::InvalidReservationId(*self));
        }
        Ok(())
    }
}

pub trait ToSql {
    fn to_sql(&self) -> String;
}
