use std::task::Poll;

use crate::{ReservationStream, RsvpService, TonicReceiverStream};
use abi::{
    reservation_service_server::ReservationService, CancelRequest, CancelResponse, Config,
    ConfirmRequest, ConfirmResponse, FilterRequest, FilterResponse, GetRequest, GetResponse,
    ListenRequest, QueryRequest, ReserveRequest, ReserveResponse, UpdateRequest, UpdateResponse,
};
use futures::Stream;
use reservation::{ReservationManager, Rsvp};
use tokio::sync::mpsc;

use tonic::{Request, Response, Status};
use tracing::info;

impl RsvpService {
    pub async fn new(config: Config) -> Result<Self, anyhow::Error> {
        Ok(Self {
            manager: ReservationManager::from_config(&config.db).await?,
        })
    }
}

impl RsvpService {
    pub async fn from_config(config: &Config) -> Result<Self, anyhow::Error> {
        Ok(Self {
            manager: ReservationManager::from_config(&config.db).await?,
        })
    }
}

#[tonic::async_trait]
impl ReservationService for RsvpService {
    /// make a reservation
    async fn reserve(
        &self,
        request: Request<ReserveRequest>,
    ) -> Result<Response<ReserveResponse>, Status> {
        let request = request.into_inner();
        if request.reservation.is_none() {
            return Err(Status::invalid_argument("reservation is required"));
        }
        let reservation = request.reservation.unwrap();
        let reservation = self.manager.reserve(reservation).await?;
        Ok(Response::new(ReserveResponse {
            reservation: Some(reservation),
        }))
    }
    /// confirm a pending reservation,if reservation is not pending, do nothing
    async fn confirm(
        &self,
        request: Request<ConfirmRequest>,
    ) -> Result<Response<ConfirmResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.change_status(request.id).await?;
        Ok(Response::new(ConfirmResponse {
            reservation: Some(reservation),
        }))
    }
    /// update a reservation note
    async fn update(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.update_note(request.id, request.note).await?;
        Ok(Response::new(UpdateResponse {
            reservation: Some(reservation),
        }))
    }
    ///cancel a reservation by id
    async fn cancel(
        &self,
        request: Request<CancelRequest>,
    ) -> Result<Response<CancelResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.delete(request.id).await?;
        Ok(Response::new(CancelResponse {
            reservation: Some(reservation),
        }))
    }
    /// get a reservation by id
    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.get(request.id).await?;
        Ok(Response::new(GetResponse {
            reservation: Some(reservation),
        }))
    }
    ///Server streaming response type for the query method.
    type queryStream = ReservationStream;
    /// query reservations by resource_id, user_id, status, start time, end time
    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::queryStream>, Status> {
        let request = request.into_inner();
        if request.query.is_none() {
            return Err(Status::invalid_argument("query is required"));
        }
        info!("query request: {:#?}", request);
        let query = request.query.unwrap();

        let rsvps = self.manager.query(query).await;
        info!("query result: {:#?}", rsvps);
        let stream = TonicReceiverStream::new(rsvps);
        Ok(Response::new(Box::pin(stream)))
    }
    /// query reservations ,order by reservation id
    async fn filter(
        &self,
        request: Request<FilterRequest>,
    ) -> Result<Response<FilterResponse>, Status> {
        let request = request.into_inner();
        if request.filter.is_none() {
            return Err(Status::invalid_argument("missing filter parameter"));
        }
        let filter = request.filter.unwrap();
        let (pager, reservations) = self.manager.filter(filter).await?;
        Ok(Response::new(FilterResponse {
            reservations,
            pager: Some(pager),
        }))
    }
    ///Server streaming response type for the listen method.
    type listenStream = ReservationStream;
    /// another system could monitor newly added/updated/cancelled/confirmed reservations
    async fn listen(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::listenStream>, Status> {
        let _request = request.into_inner();
        // let reservations = self.manager.listen(request).await?;
        todo!()
    }
}

impl<T> TonicReceiverStream<T> {
    pub fn new(inner: mpsc::Receiver<Result<T, abi::Error>>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for TonicReceiverStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.inner.poll_recv(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(Ok(item))),
            Poll::Ready(Some(Err(err))) => {
                Poll::Ready(Some(Err(Status::internal(err.to_string()))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

// test
#[cfg(test)]
mod tests {

    use abi::{convert_to_timestamp, ReservationFilter, ReservationQuery, ReservationStatus};
    use futures::{future, TryStreamExt};
    use tonic::Code;
    use tracing::log::info;

    use crate::test_utils::TestConfig;

    use super::*;

    #[tokio::test]
    async fn rpc_reserve_should_work() {
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let request = ReserveRequest {
            reservation: Some(abi::Reservation::new_pending(
                "aliceid",
                "test-room-317",
                "2022-12-25T15:00:00-0700".parse().unwrap(),
                "2022-12-27T12:00:00-0700".parse().unwrap(),
                "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
            )),
        };
        let response = service.reserve(Request::new(request)).await.unwrap();
        let reservation_res = response.into_inner().reservation;
        assert!(reservation_res.is_some());
    }
    #[tokio::test]
    async fn rpc_confirm_should_work() {
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let request = ReserveRequest {
            reservation: Some(abi::Reservation::new_pending(
                "aliceid",
                "test-room-317",
                "2022-12-25T15:00:00-0700".parse().unwrap(),
                "2022-12-27T12:00:00-0700".parse().unwrap(),
                "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
            )),
        };
        service.reserve(Request::new(request)).await.unwrap();
        let request = ConfirmRequest { id: 1 };
        let response = service.confirm(Request::new(request)).await.unwrap();
        let reservation_res = response.into_inner().reservation;
        assert!(reservation_res.is_some());
    }
    #[tokio::test]
    async fn rpc_update_should_work() {
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let request = ReserveRequest {
            reservation: Some(abi::Reservation::new_pending(
                "aliceid",
                "test-room-317",
                "2022-12-25T15:00:00-0700".parse().unwrap(),
                "2022-12-27T12:00:00-0700".parse().unwrap(),
                "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
            )),
        };
        service.reserve(Request::new(request)).await.unwrap();
        let request = UpdateRequest {
            id: 1,
            note: "I'll arrive at 4pm. Please help to upgrade to execuitive room if possible."
                .to_string(),
        };
        let response = service.update(Request::new(request)).await.unwrap();
        let reservation_res = response.into_inner().reservation;
        assert!(reservation_res.is_some());
    }
    //cancel a reservation by id
    #[tokio::test]
    async fn rpc_cancel_should_work() {
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let request = ReserveRequest {
            reservation: Some(abi::Reservation::new_pending(
                "aliceid",
                "test-room-317",
                "2022-12-25T15:00:00-0700".parse().unwrap(),
                "2022-12-27T12:00:00-0700".parse().unwrap(),
                "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
            )),
        };
        service.reserve(Request::new(request)).await.unwrap();
        let request = CancelRequest { id: 1 };
        let response = service.cancel(Request::new(request)).await.unwrap();
        let reservation_res = response.into_inner().reservation;
        assert!(reservation_res.is_some());
    }
    //get a reservation by id
    #[tokio::test]
    async fn rpc_get_should_work() {
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let request = ReserveRequest {
            reservation: Some(abi::Reservation::new_pending(
                "aliceid",
                "test-room-317",
                "2022-12-25T15:00:00-0700".parse().unwrap(),
                "2022-12-27T12:00:00-0700".parse().unwrap(),
                "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
            )),
        };
        service.reserve(Request::new(request)).await.unwrap();
        let request = GetRequest { id: 1 };
        let response = service.get(Request::new(request)).await.unwrap();
        let reservation_res = response.into_inner().reservation;
        assert!(reservation_res.is_some());
    }
    //query reservations by filter
    #[tokio::test]
    async fn rpc_filter_should_work() {
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let request = ReserveRequest {
            reservation: Some(abi::Reservation::new_pending(
                "aliceid",
                "test-room-317",
                "2022-12-25T15:00:00-0700".parse().unwrap(),
                "2022-12-27T12:00:00-0700".parse().unwrap(),
                "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
            )),
        };
        service.reserve(Request::new(request)).await.unwrap();
        let request = FilterRequest {
            filter: Some(ReservationFilter {
                resource_id: "test-room-317".to_string(),
                page_size: 10,
                desc: false,
                ..Default::default()
            }),
        };
        let response = service.filter(Request::new(request)).await.unwrap();
        let reservations = response.into_inner().reservations;
        assert!(!reservations.is_empty());
    }
    //query reservations by filter
    #[tokio::test]
    async fn rpc_query_should_work() {
        let subscriber = tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let request = ReserveRequest {
            reservation: Some(abi::Reservation::new_pending(
                "aliceid",
                "test-room-317",
                "2022-12-25T15:00:00-0700".parse().unwrap(),
                "2022-12-27T12:00:00-0700".parse().unwrap(),
                "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
            )),
        };
        service.reserve(Request::new(request)).await.unwrap();
        let request = QueryRequest {
            query: Some(ReservationQuery {
                resource_id: "test-room-317".to_string(),
                start: Some(convert_to_timestamp(
                    "2022-12-25T15:00:00-0700".parse().unwrap(),
                )),
                end: Some(convert_to_timestamp(
                    "2022-12-27T12:00:00-0700".parse().unwrap(),
                )),
                status: ReservationStatus::Pending as i32,
                ..Default::default()
            }),
        };
        let query = service.query(Request::new(request)).await.unwrap();
        let stream = query.into_inner();
        let fut = stream
            .try_for_each(|res| {
                info!("rpc_query_should_work reservation: {:?}", res);
                let id = res.id;
                if id > 0 {
                    future::ok(())
                } else {
                    future::err(Status::new(Code::Internal, "query reservations failed"))
                }
            })
            .await;
        assert!(fut.is_ok());
        // assert!(!reservations.is_empty());
    }
}
