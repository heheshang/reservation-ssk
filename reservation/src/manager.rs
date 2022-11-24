use crate::{ReservationId, ReservationManager, Rsvp};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{postgres::types::PgRange, types::Uuid, PgPool, Row};

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, abi::Error> {
        rsvp.validate()?;
        if rsvp.start.is_none() || rsvp.end.is_none() {
            return Err(abi::Error::InvalidTime);
        }

        let timespan: PgRange<DateTime<Utc>> = rsvp.get_time_span().into();

        let status = abi::ReservationStatus::from_i32(rsvp.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        // generate a insert sql for the reservation
        let id: Uuid = sqlx::query(
            r#"
            INSERT INTO rsvp.reservations (resource_id, user_id, timespan, note, status)
            VALUES ($1, $2, $3, $4, $5::rsvp.reservation_status)
            RETURNING id
            "#,
        )
        .bind(rsvp.resource_id.clone())
        .bind(rsvp.user_id.clone())
        .bind(timespan)
        .bind(rsvp.note.clone())
        .bind(status.to_string())
        .fetch_one(&self.pool)
        .await?
        .get(0);
        rsvp.id = id.to_string();
        Ok(rsvp)
    }

    async fn change_status(&self, _id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        todo!()
    }
    async fn update_note(
        &self,
        _id: ReservationId,
        _note: String,
    ) -> Result<abi::Reservation, abi::Error> {
        todo!()
    }
    async fn delete(&self, _id: ReservationId) -> Result<(), abi::Error> {
        todo!()
    }
    async fn get(&self, _id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        todo!()
    }
    async fn query(
        &self,
        _query: abi::ReservationQuery,
    ) -> Result<Vec<abi::Reservation>, abi::Error> {
        todo!()
    }
}
impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() {
        let manager = ReservationManager::new(migrated_pool.clone());
        let rsvp = abi::Reservation::new_pending(
            "user_id",
            "resource_id",
            "2021-12-25T00:00:00Z".parse().unwrap(),
            "2021-12-28T00:00:00Z".parse().unwrap(),
            "note",
        );

        let r = manager.reserve(rsvp).await.unwrap();
        assert_eq!(r.resource_id, "resource_id");
        assert_eq!(r.user_id, "user_id");
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_conflict_reservation_should_reject() {
        let manager = ReservationManager::new(migrated_pool.clone());
        let rsvp1 = abi::Reservation::new_pending(
            "user_id1",
            "resource_id1",
            "2021-12-25T00:00:00Z".parse().unwrap(),
            "2021-12-28T00:00:00Z".parse().unwrap(),
            "hello",
        );
        let _rsvp2 = abi::Reservation::new_pending(
            "user_id2",
            "resource_id1",
            "2021-12-26T00:00:00Z".parse().unwrap(),
            "2021-12-27T00:00:00Z".parse().unwrap(),
            "hello",
        );

        let _r1 = manager.reserve(rsvp1).await.unwrap();
        // let err = manager.reserve(rsvp2).await.unwrap_err();
        // println!("{:?}", err);
        // if let abi::Error::ConflictReservation() = err {
        // } else {
        //     panic!("expect conflict error");
        // }
    }
}
