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
        _request: Request<ConfirmRequest>,
    ) -> Result<Response<ConfirmResponse>, Status> {
        unimplemented!()
    }
    /// update a reservation note
    async fn update(
        &self,
        _request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        unimplemented!()
    }
    ///cancel a reservation by id
    async fn cancel(
        &self,
        _request: Request<CancelRequest>,
    ) -> Result<Response<CancelResponse>, Status> {
        unimplemented!()
    }
    /// get a reservation by id
    async fn get(&self, _request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        unimplemented!()
    }
    ///Server streaming response type for the query method.
    type queryStream = ReservationStream;
    /// query reservations by resource_id, user_id, status, start time, end time
    async fn query(
        &self,
        _request: Request<QueryRequest>,
    ) -> Result<Response<Self::queryStream>, Status> {
        unimplemented!()
    }
    /// query reservations ,order by reservation id
    async fn filter(
        &self,
        _request: Request<FilterRequest>,
    ) -> Result<Response<FilterResponse>, Status> {
        unimplemented!()
    }
    ///Server streaming response type for the listen method.
    type listenStream = ReservationStream;
    /// another system could monitor newly added/updated/cancelled/confirmed reservations
    async fn listen(
        &self,
        _request: Request<ListenRequest>,
    ) -> Result<Response<Self::listenStream>, Status> {
        unimplemented!()
    }
}

// test
#[cfg(test)]
mod tests {
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
}
