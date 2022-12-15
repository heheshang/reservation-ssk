use crate::{ReservationManager, Rsvp};
use abi::{DbConfig, FilterPager, ReservationId, Validator};
use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

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
    ) -> Result<Vec<abi::Reservation>, abi::Error> {
        query.validate()?;
        let user_id = string_to_option(&query.user_id);
        let resource_id = string_to_option(&query.resource_id);
        let during = query.get_query_timespan();
        let status = abi::ReservationStatus::from_i32(query.status)
            .unwrap_or(abi::ReservationStatus::Pending)
            .to_string();
        let resps = sqlx::query_as(
            r#"SELECT * FROM rsvp.query($1, $2, $3, $4::rsvp.reservation_status, $5, $6, $7)"#,
        )
        .bind(user_id)
        .bind(resource_id)
        .bind(during)
        .bind(status)
        .bind(query.page)
        .bind(query.desc)
        .bind(query.page_size)
        .fetch_all(&self.pool)
        .await?;
        Ok(resps)
    }

    async fn filter(
        &self,
        filter: abi::ReservationFilter,
    ) -> Result<(FilterPager, Vec<abi::Reservation>), abi::Error> {
        // filter reservations by user_id, resource_id, during, status and order by id
        let user_id = string_to_option(&filter.user_id);
        let resource_id = string_to_option(&filter.resource_id);
        let status = abi::ReservationStatus::from_i32(filter.status)
            .unwrap_or(abi::ReservationStatus::Pending)
            .to_string();

        let page_size = if filter.page_size < 10 || filter.page_size > 100 {
            10
        } else {
            filter.page_size
        };

        let rsvps: Vec<abi::Reservation> = sqlx::query_as(
            r#"SELECT * FROM rsvp.filter($1, $2, $3::rsvp.reservation_status, $4, $5, $6)"#,
        )
        .bind(user_id)
        .bind(resource_id)
        .bind(status)
        .bind(filter.cursor)
        .bind(filter.desc)
        .bind(page_size)
        .fetch_all(&self.pool)
        .await?;
        // if the first id is current cursor, then we have prev,we start form 1
        // if len-start > page_size, then we have next, we end at len-1

        let has_prev = !rsvps.is_empty() && rsvps[0].id == filter.cursor;
        let start = usize::from(has_prev);

        let has_next = rsvps.len() - start > page_size as usize;
        let end = if has_next {
            rsvps.len() - 1
        } else {
            rsvps.len()
        };

        let prev = if has_prev { rsvps[start - 1].id } else { -1 };
        let next = if has_next { rsvps[end - 1].id } else { -1 };

        let results = rsvps[start..end].to_vec();
        let pager = FilterPager {
            prev,
            next,
            total: 0,
        };
        Ok((pager, results))
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

fn string_to_option(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.into())
    }
}

#[cfg(test)]
mod tests {

    use abi::{
        Reservation, ReservationConflict, ReservationConflictInfo, ReservationFilterBuilder,
        ReservationQueryBuilder, ReservationWindow,
    };
    use prost_types::Timestamp;

    use super::*;
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() {
        let (rsvp, _) = make_ssk_reservation(migrated_pool).await;
        println!("{:?}", rsvp);
        assert_eq!(rsvp.resource_id, "ocean-view-room-713");
        assert_eq!(rsvp.user_id, "sskid");
        assert_eq!(rsvp.id, 1);
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
        assert_eq!(err, abi::Error::ConflictReservation(info));
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn change_status_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool).await;
        println!("rsvp: {:?}", rsvp);
        let id = rsvp.id;
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
        let r = manager.get(rsvp.id).await.unwrap();
        assert_eq!(r.id, rsvp.id);
    }

    // // query function test
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn query_reservations_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;
        let query = ReservationQueryBuilder::default()
            .user_id("aliceid")
            .status(abi::ReservationStatus::Pending as i32)
            .start("2022-12-25T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2023-02-25T12:00:00-0700".parse::<Timestamp>().unwrap())
            .build()
            .unwrap();
        let rsvps = manager.query(query).await.unwrap();
        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvp, rsvps[0]);

        let query = ReservationQueryBuilder::default()
            .user_id("aliceid")
            .status(abi::ReservationStatus::Confirmed as i32)
            .start("2022-12-25T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2023-02-21T12:00:00-0700".parse::<Timestamp>().unwrap())
            .build()
            .unwrap();

        let rsvps = manager.query(query).await.unwrap();
        assert_eq!(rsvps.len(), 0);

        manager.change_status(rsvp.id).await.unwrap();
        let query = ReservationQueryBuilder::default()
            .user_id("aliceid")
            .status(abi::ReservationStatus::Confirmed as i32)
            .start("2022-12-25T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2023-02-25T12:00:00-0700".parse::<Timestamp>().unwrap())
            .build()
            .unwrap();

        let rsvps = manager.query(query).await.unwrap();
        assert_eq!(rsvps.len(), 1);
    }

    // test filter function
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn filter_reservations_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;
        let filter = ReservationFilterBuilder::default()
            .user_id("aliceid")
            .status(abi::ReservationStatus::Pending as i32)
            .build()
            .unwrap();
        let (pager, rsvps) = manager.filter(filter).await.unwrap();
        assert_eq!(pager.prev, -1);
        assert_eq!(pager.next, -1);
        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvp, rsvps[0]);
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
