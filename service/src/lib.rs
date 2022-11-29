use std::pin::Pin;

use abi::Reservation;
use futures::Stream;
use reservation::ReservationManager;
use tonic::Status;

mod service;
#[cfg(test)]
pub mod test_utils;

pub struct RsvpService {
    manager: ReservationManager,
}

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, Status>> + Send>>;
