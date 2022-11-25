use crate::{ReservationId, ReservationManager, Rsvp};
use abi::convert_to_utc_time;
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

    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        // if current status is `pending`, then change to `confirmed` otherwise do nothing
        let id: Uuid =
            Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;

        let rsvp: abi::Reservation = sqlx::query_as(
            r#"
            UPDATE rsvp.reservations
            SET status = CASE WHEN status = 'pending' THEN 'confirmed' ELSE status END
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(rsvp)
    }
    async fn update_note(
        &self,
        id: ReservationId,
        note: String,
    ) -> Result<abi::Reservation, abi::Error> {
        let id: Uuid =
            Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;

        let rsvp: abi::Reservation = sqlx::query_as(
            r#"
            UPDATE rsvp.reservations
            SET note = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(note)
        .fetch_one(&self.pool)
        .await?;
        Ok(rsvp)
    }
    async fn delete(&self, id: ReservationId) -> Result<(), abi::Error> {
        let id: Uuid =
            Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;

        sqlx::query(
            r#"
            DELETE FROM rsvp.reservations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        let id: Uuid =
            Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;

        let rsvp: abi::Reservation = sqlx::query_as(
            r#"
            SELECT * FROM rsvp.reservations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(rsvp)
    }
    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> Result<Vec<abi::Reservation>, abi::Error> {
        let mut sql = "SELECT * FROM rsvp.reservations WHERE 1 = 1".to_string();
        let mut params = Vec::new();

        if let Some(resource_id) = query.resource_id {
            sql.push_str(" AND resource_id = $1");
            params.push(resource_id);
        }
        if let Some(user_id) = query.user_id {
            sql.push_str(" AND user_id = $2");
            params.push(user_id);
        }
        if let Some(status) = query.status {
            sql.push_str(" AND status = $3::rsvp.reservation_status");
            params.push(status.to_string());
        }
        if let Some(start) = query.start {
            sql.push_str(" AND timespan @> $4");
            params.push(convert_to_utc_time(start).to_string());
        }
        if let Some(end) = query.end {
            sql.push_str(" AND timespan @< $5");
            params.push(convert_to_utc_time(end).to_string());
        }

        let rsvps: Vec<abi::Reservation> = sqlx::query_as(&sql)
            .bind(params)
            .fetch_all(&self.pool)
            .await?;
        Ok(rsvps)
    }
}
impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(test)]
mod tests {

    use abi::{Reservation, ReservationConflict, ReservationConflictInfo, ReservationWindow};

    use super::*;
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() {
        let (rsvp, _) = make_ssk_reservation(migrated_pool).await;
        assert_eq!(rsvp.resource_id, "ocean-view-room-713");
        assert_eq!(rsvp.user_id, "sskid");
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_conflict_reservation_should_reject() {
        let (rsvp, manager) = make_ssk_reservation(migrated_pool).await;
        let rsvp2 = abi::Reservation::new_pending(
            "aliceid",
            "ocean-view-room-713",
            "2022-12-25T15:00:00-0700".parse().unwrap(),
            "2022-12-27T12:00:00-0700".parse().unwrap(),
            "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
        );
        println!("rsvp: {:?}", rsvp);
        let err = manager.reserve(rsvp2).await.unwrap_err();
        println!("err {:?}", err);
        let info = ReservationConflictInfo::Parsed(ReservationConflict {
            new: ReservationWindow {
                rid: "ocean-view-room-713".to_string(),
                start: "2022-12-25T15:00:00-0700".parse().unwrap(),
                end: "2022-12-27T12:00:00-0700".parse().unwrap(),
            },
            old: ReservationWindow {
                rid: "ocean-view-room-713".to_string(),
                start: "2022-12-25T15:00:00-0700".parse().unwrap(),
                end: "2022-12-28T12:00:00-0700".parse().unwrap(),
            },
        });
        println!("info {:?}", info);
        assert_eq!(err, abi::Error::ReservationConflict(info));
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn change_status_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool).await;
        println!("rsvp: {:?}", rsvp);
        let id = rsvp.id.clone();
        let rsvp = manager.change_status(id).await.unwrap();
        assert_eq!(rsvp.status, abi::ReservationStatus::Confirmed as i32);
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_change_status_not_pending_shhou_do_nothing() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool).await;
        println!("rsvp: {:?}", rsvp);
        let rsvp = manager.change_status(rsvp.id).await.unwrap();
        // change status again should do nothing
        let rsvp = manager.change_status(rsvp.id).await.unwrap();
        assert_eq!(rsvp.status, abi::ReservationStatus::Confirmed as i32);
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn update_note_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;
        println!("r: {:?}", rsvp);
        let r = manager
            .update_note(rsvp.id, "new note".to_string())
            .await
            .unwrap();
        assert_eq!(r.note, "new note");
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn delete_reservation_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;
        println!("r: {:?}", rsvp);
        let r = manager.delete(rsvp.id).await;
        assert!(r.is_ok());
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn get_reservation_by_id_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;
        println!("r: {:?}", rsvp);
        let r = manager.get(rsvp.id.clone()).await.unwrap();
        assert_eq!(r.id, rsvp.id);
    }

    // private none test functions
    async fn make_ssk_reservation(pool: PgPool) -> (Reservation, ReservationManager) {
        make_reservation(
            pool,
            "sskid",
            "ocean-view-room-713",
            "2022-12-25T15:00:00-0700",
            "2022-12-28T12:00:00-0700",
            "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
        )
        .await
    }

    async fn make_alice_reservation(pool: PgPool) -> (Reservation, ReservationManager) {
        make_reservation(
            pool,
            "aliceid",
            "ixia-test-1",
            "2023-01-25T15:00:00-0700",
            "2023-02-25T12:00:00-0700",
            "I need to book this for xyz project for a month.",
        )
        .await
    }

    async fn make_reservation(
        pool: PgPool,
        uid: &str,
        rid: &str,
        start: &str,
        end: &str,
        note: &str,
    ) -> (Reservation, ReservationManager) {
        let manager = ReservationManager::new(pool.clone());
        let rsvp = abi::Reservation::new_pending(
            uid,
            rid,
            start.parse().unwrap(),
            end.parse().unwrap(),
            note,
        );
        (manager.reserve(rsvp).await.unwrap(), manager)
    }
}
