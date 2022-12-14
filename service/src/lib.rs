use std::pin::Pin;

use abi::{reservation_service_server::ReservationServiceServer, Config, Reservation};
use futures::Stream;
use reservation::ReservationManager;
use tokio::sync::mpsc;
use tonic::{transport::Server, Status};
use tracing::info;

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

pub async fn start_server(config: &Config) -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let svc = RsvpService::from_config(config).await?;
    let svc = ReservationServiceServer::new(svc);
    info!("Starting server at {}", addr);
    Server::builder()
        .add_service(svc)
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
