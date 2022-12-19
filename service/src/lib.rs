use std::pin::Pin;

use abi::Reservation;
use futures::Stream;
use reservation::ReservationManager;
use tokio::sync::mpsc;
use tonic::Status;

mod service;
#[cfg(test)]
pub mod test_utils;

pub struct RsvpService {
    manager: ReservationManager,
}

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, Status>> + Send>>;

pub struct TonicReceiverStream<T> {
    inner: mpsc::Receiver<Result<T, abi::Error>>,
}
