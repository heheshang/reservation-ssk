mod manager;
use abi::{Error, ReservationId};
use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Debug)]
pub struct ReservationManager {
    pool: PgPool,
}

#[async_trait]
pub trait Rsvp {
    /// make a reservation
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, Error>;

    /// change a reservation status (if current status is `pending`, then change to `confirmed`)
    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, Error>;
    /// update note for a reservation
    async fn update_note(&self, id: ReservationId, note: String)
        -> Result<abi::Reservation, Error>;

    /// delete a reservation
    async fn delete(&self, id: ReservationId) -> Result<(), Error>;
    ///get a reservation by id
    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, Error>;
    /// query reservations
    async fn query(&self, query: abi::ReservationQuery) -> Result<Vec<abi::Reservation>, Error>;
}
