use abi::{
    reservation_service_server::ReservationService, CancelRequest, CancelResponse, Config,
    ConfirmRequest, ConfirmResponse, FilterRequest, FilterResponse, GetRequest, GetResponse,
    ListenRequest, QueryRequest, ReserveRequest, ReserveResponse, UpdateRequest, UpdateResponse,
};
use reservation::{ReservationManager, Rsvp};
use tonic::{Request, Response, Status};

use crate::{ReservationStream, RsvpService};

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
        let request = request.into_inner().query.unwrap();
        let _reservations = self.manager.query(request).await?;
        // Ok(Response::new(ReservationStream::new(reservations)))
        todo!()
    }
    /// query reservations ,order by reservation id
    async fn filter(
        &self,
        request: Request<FilterRequest>,
    ) -> Result<Response<FilterResponse>, Status> {
        let request = request.into_inner().filter.unwrap();
        let (pager, reservations) = self.manager.filter(request).await?;
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

// test
#[cfg(test)]
mod tests {

    use abi::ReservationFilter;

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
                cursor: 0,
                page_size: 10,
                desc: false,
                ..Default::default()
            }),
        };
        let response = service.filter(Request::new(request)).await.unwrap();
        let _reservations = response.into_inner().reservations;
        // assert!(!reservations.is_empty());
    }
}
