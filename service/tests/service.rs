#[path = "../src/test_utils.rs"]
mod test_utils;
use std::time::Duration;

use abi::{
    reservation_service_client::ReservationServiceClient, Config, FilterResponse, Reservation,
    ReservationFilterBuilder, ReservationQueryBuilder, ReserveRequest,
};
use reservation_service::start_server;
use test_utils::TestConfig;
use tokio::time;
use tonic::transport::Channel;
use tracing::info;

#[tokio::test]
async fn grpc_server_should_work() {
    let config = TestConfig::with_server_port(50000);
    let mut client = get_test_client(&config).await;
    // make a reservation
    let mut rsvp = Reservation::new_pending(
        "tyr",
        "ixia-3230",
        "2022-12-26T15:00:00-0700".parse().unwrap(),
        "2022-12-30T12:00:00-0700".parse().unwrap(),
        "test device reservation",
    );
    let ret = client
        .reserve(ReserveRequest::new(rsvp.clone()))
        .await
        .unwrap()
        .into_inner()
        .reservation
        .unwrap();
    rsvp.id = ret.id;
    assert_eq!(rsvp, ret);
    // then we try to make a conflicting reservation
    let rsvp2 = Reservation::new_pending(
        "tyr",
        "ixia-3230",
        "2022-12-26T15:00:00-0700".parse().unwrap(),
        "2022-12-30T12:00:00-0700".parse().unwrap(),
        "test device reservation",
    );
    let ret = client.reserve(ReserveRequest::new(rsvp2.clone())).await;
    assert!(ret.is_err());

    // then we confirm the first reservation
    let ret = client
        .confirm(abi::ConfirmRequest::new(rsvp.id))
        .await
        .unwrap()
        .into_inner()
        .reservation
        .unwrap();
    assert_eq!(ret.status, abi::ReservationStatus::Confirmed as i32);
}

#[tokio::test]
async fn grpc_query_should_work() {
    let tconfig = TestConfig::with_server_port(50001);
    let mut client = get_test_client(&tconfig).await;
    make_reservation(&mut client, 10).await;

    let query = ReservationQueryBuilder::default()
        .user_id("alice")
        .build()
        .unwrap();
    let mut ret = client
        .query(abi::QueryRequest::new(query))
        .await
        .unwrap()
        .into_inner();
    while let Some(r) = ret.message().await.unwrap() {
        info!("{:?}", r);
    }
}
#[tokio::test]
async fn grpc_filter_should_work() {
    let tconfig = TestConfig::with_server_port(50002);
    let mut client = get_test_client(&tconfig).await;
    make_reservation(&mut client, 100).await;

    let filter = ReservationFilterBuilder::default()
        .user_id("alice")
        .status(abi::ReservationStatus::Pending as i32)
        .build()
        .unwrap();
    let FilterResponse {
        pager,
        reservations,
    } = client
        .filter(abi::FilterRequest::new(filter.clone()))
        .await
        .unwrap()
        .into_inner();

    let pager = pager.unwrap();
    assert_eq!(pager.next, filter.page_size);
    assert_eq!(pager.prev, -1);

    assert_eq!(reservations.len(), filter.page_size as usize);
    let mut next_filter = filter.clone();
    next_filter.cursor = pager.next;
}
async fn get_test_client(tconfig: &TestConfig) -> ReservationServiceClient<Channel> {
    let config = &tconfig.config;
    setup_server(config).await;
    ReservationServiceClient::connect(config.server.url(false))
        .await
        .unwrap()
}

async fn setup_server(config: &Config) {
    let config_cloned = config.clone();
    tokio::spawn(async move {
        start_server(&config_cloned).await.unwrap();
    });
    time::sleep(Duration::from_millis(1000)).await;
}

async fn make_reservation(client: &mut ReservationServiceClient<Channel>, count: u32) {
    for i in 0..count {
        let mut rsvp = Reservation::new_pending(
            "alice",
            format!("router-{}", i),
            "2022-12-26T15:00:00-0700".parse().unwrap(),
            "2022-12-30T12:00:00-0700".parse().unwrap(),
            format!("test device reservation {}", i),
        );
        let ret = client
            .reserve(ReserveRequest::new(rsvp.clone()))
            .await
            .unwrap()
            .into_inner()
            .reservation
            .unwrap();
        rsvp.id = ret.id;
        assert_eq!(rsvp, ret);
    }
}
