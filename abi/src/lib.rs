mod error;
mod pb;
mod types;
mod utils;
pub use error::*;
pub use pb::*;
pub use types::*;
pub use utils::*;
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
