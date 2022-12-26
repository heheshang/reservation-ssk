use crate::{ReservationManager, Rsvp};
use abi::{DbConfig, FilterPager, Normalizer, ReservationId, ToSql, Validator};
use async_trait::async_trait;
use futures::StreamExt;
use sqlx::{postgres::PgPoolOptions, Either, PgPool, Row};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, abi::Error> {
        rsvp.validate()?;
        if rsvp.start.is_none() || rsvp.end.is_none() {
            return Err(abi::Error::InvalidTime);
        }

        let timespan = rsvp.get_time_span();

        let status = abi::ReservationStatus::from_i32(rsvp.status)
            .unwrap_or(abi::ReservationStatus::Pending);
        info!("timespan: {:?}", timespan);
        // generate a insert sql for the reservation
        let id = sqlx::query(
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
        rsvp.id = id;
        Ok(rsvp)
    }

    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        // if current status is `pending`, then change to `confirmed` otherwise do nothing
        id.validate()?;

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
        id.validate()?;
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
    /// 删除并返回old row
    async fn delete(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        id.validate()?;
        let rsvp: abi::Reservation = sqlx::query_as(
            r#"
            DELETE FROM rsvp.reservations
            WHERE id = $1 RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(rsvp)
    }
    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        id.validate()?;
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
    ) -> mpsc::Receiver<Result<abi::Reservation, abi::Error>> {
        let pool = self.pool.clone();
        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            let sql = query.to_sql();
            let mut rsvps = sqlx::query_as(&sql).fetch_many(&pool);

            while let Some(ret) = rsvps.next().await {
                match ret {
                    Ok(Either::Left(r)) => {
                        info!("Query Result: {:?}", r);
                    }
                    Ok(Either::Right(r)) => {
                        if tx.send(Ok(r)).await.is_err() {
                            error!("Failed to send reservation");
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to query reservation: {}", e);
                        if tx.send(Err(e.into())).await.is_err() {
                            error!("Failed to send reservation");
                            break;
                        }
                    }
                }
            }
        });
        rx
    }

    async fn filter(
        &self,
        mut filter: abi::ReservationFilter,
    ) -> Result<(FilterPager, Vec<abi::Reservation>), abi::Error> {
        filter.normalize()?;

        let sql = filter.to_sql();
        let rsvps: Vec<abi::Reservation> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;
        let mut rsvps = rsvps.into_iter().collect();

        let pager = filter.get_pager(&mut rsvps);
        Ok((pager, rsvps.into_iter().collect()))
    }
}
impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn from_config(config: &DbConfig) -> Result<Self, abi::Error> {
        let pool = PgPoolOptions::default()
            .max_connections(config.max_connections)
            .connect(&config.url())
            .await?;
        Ok(Self::new(pool))
    }
}

#[cfg(test)]
mod tests {

    use abi::{
        Reservation, ReservationConflict, ReservationConflictInfo, ReservationFilterBuilder,
        ReservationQueryBuilder, ReservationWindow,
    };
    use prost_types::Timestamp;
    use sqlx_db_tester::TestDb;

    use super::*;
    #[tokio::test]
    async fn reserve_should_work_for_valid_window() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, _) = make_ssk_reservation(pool).await;
        println!("{:?}", rsvp);
        assert_eq!(rsvp.resource_id, "ocean-view-room-713");
        assert_eq!(rsvp.user_id, "sskid");
        assert_eq!(rsvp.id, 1);
    }

    #[tokio::test]
    async fn reserve_conflict_reservation_should_reject() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_ssk_reservation(pool).await;
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
        assert_eq!(err, abi::Error::ConflictReservation(info));
    }

    #[tokio::test]
    async fn change_status_should_work() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_alice_reservation(pool).await;
        println!("rsvp: {:?}", rsvp);
        let id = rsvp.id;
        let rsvp = manager.change_status(id).await.unwrap();
        assert_eq!(rsvp.status, abi::ReservationStatus::Confirmed as i32);
    }

    #[tokio::test]
    async fn reserve_change_status_not_pending_shhou_do_nothing() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_alice_reservation(pool).await;
        println!("rsvp: {:?}", rsvp);
        let rsvp = manager.change_status(rsvp.id).await.unwrap();
        // change status again should do nothing
        let rsvp = manager.change_status(rsvp.id).await.unwrap();
        assert_eq!(rsvp.status, abi::ReservationStatus::Confirmed as i32);
    }

    #[tokio::test]
    async fn update_note_should_work() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_alice_reservation(pool.clone()).await;
        println!("r: {:?}", rsvp);
        let r = manager
            .update_note(rsvp.id, "new note".to_string())
            .await
            .unwrap();
        assert_eq!(r.note, "new note");
    }
    #[tokio::test]
    async fn delete_reservation_should_work() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_alice_reservation(pool.clone()).await;
        println!("r: {:?}", rsvp);
        let r = manager.delete(rsvp.id).await;
        assert!(r.is_ok());
    }
    #[tokio::test]
    async fn get_reservation_by_id_should_work() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_alice_reservation(pool.clone()).await;
        println!("r: {:?}", rsvp);
        let r = manager.get(rsvp.id).await.unwrap();
        assert_eq!(r.id, rsvp.id);
    }

    // // query function test
    #[tokio::test]
    async fn query_reservations_should_work() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_alice_reservation(pool.clone()).await;
        let query = ReservationQueryBuilder::default()
            .user_id("aliceid")
            .start("2021-11-01T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2023-12-31T12:00:00-0700".parse::<Timestamp>().unwrap())
            .status(abi::ReservationStatus::Pending as i32)
            .build()
            .unwrap();

        let mut rx = manager.query(query).await;
        assert_eq!(rx.recv().await, Some(Ok(rsvp.clone())));
        assert_eq!(rx.recv().await, None);

        // if window is not in range, should return empty
        let query = ReservationQueryBuilder::default()
            .user_id("aliceid")
            .start("2023-01-01T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2023-02-01T12:00:00-0700".parse::<Timestamp>().unwrap())
            .status(abi::ReservationStatus::Confirmed as i32)
            .build()
            .unwrap();
        let mut rx = manager.query(query).await;
        assert_eq!(rx.recv().await, None);

        // if status is not in correct, should return empty
        let query = ReservationQueryBuilder::default()
            .user_id("aliceid")
            .start("2021-11-01T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2023-12-31T12:00:00-0700".parse::<Timestamp>().unwrap())
            .status(abi::ReservationStatus::Confirmed as i32)
            .build()
            .unwrap();
        let mut rx = manager.query(query.clone()).await;
        assert_eq!(rx.recv().await, None);

        // change state to confirmed, query should get result
        let rsvp = manager.change_status(rsvp.id).await.unwrap();
        let mut rx = manager.query(query).await;
        assert_eq!(rx.recv().await, Some(Ok(rsvp)));
    }

    // test filter function
    #[tokio::test]
    async fn filter_reservations_should_work() {
        let tdb = get_db();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_alice_reservation(pool.clone()).await;
        let filter = ReservationFilterBuilder::default()
            .user_id("aliceid")
            .status(abi::ReservationStatus::Pending as i32)
            .build()
            .unwrap();
        let (pager, rsvps) = manager.filter(filter).await.unwrap();
        assert_eq!(pager.prev, None);
        assert_eq!(pager.next, None);
        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvp, rsvps[0]);
    }

    fn get_db() -> TestDb {
        TestDb::new(
            "localhost",
            15432,
            "postgres",
            "7cOPpA7dnc",
            "../migrations",
        )
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
