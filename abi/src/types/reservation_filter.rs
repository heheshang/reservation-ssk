use std::collections::VecDeque;

use crate::{
    pager::{Id, PageInfo, Pager, Paginator},
    Error, FilterPager, Normalizer, ReservationFilter, ReservationFilterBuilder, ReservationStatus,
    ToSql, Validator,
};

impl ReservationFilterBuilder {
    pub fn build(&self) -> Result<ReservationFilter, Error> {
        let mut filter = self
            .private_build()
            .expect("failed to build reservation filter");
        filter.normalize()?;
        Ok(filter)
    }
}

impl Validator for ReservationFilter {
    fn validate(&self) -> Result<(), Error> {
        if self.page_size < 10 || self.page_size > 100 {
            return Err(Error::InvalidPageSize(self.page_size));
        }
        if let Some(cursor) = self.cursor {
            if cursor <= 0 {
                return Err(Error::InvalidCursor(cursor));
            }
        }
        ReservationStatus::from_i32(self.status).ok_or(Error::InvalidStatus(self.status))?;
        Ok(())
    }
}
impl Normalizer for ReservationFilter {
    fn do_normalize(&mut self) {
        if self.status == ReservationStatus::Unknown as i32 {
            self.status = ReservationStatus::Pending as i32;
        }
    }
}

impl From<Pager> for FilterPager {
    fn from(pager: Pager) -> Self {
        Self {
            prev: pager.prev,
            next: pager.next,
            total: pager.total,
        }
    }
}

impl From<&FilterPager> for Pager {
    fn from(pager: &FilterPager) -> Self {
        Self {
            prev: pager.prev,
            next: pager.next,
            total: pager.total,
        }
    }
}

impl ReservationFilter {
    pub fn get_pager<T: Id>(&self, data: &mut VecDeque<T>) -> FilterPager {
        let page_info = self.page_info();
        let pager = page_info.get_pager(data);
        pager.into()
    }
    pub fn get_cursor(&self) -> i64 {
        self.cursor.unwrap_or(if self.desc { i64::MAX } else { 0 })
    }
    pub fn get_status(&self) -> ReservationStatus {
        ReservationStatus::from_i32(self.status).unwrap()
    }

    pub fn next_page(&self, pager: &FilterPager) -> Option<Self> {
        let page_info = self.page_info();
        let pager = pager.into();
        let page_info = page_info.next_page(&pager);

        page_info.map(|page_info| Self {
            cursor: page_info.cursor,
            page_size: page_info.page_size,
            desc: page_info.desc,
            status: self.status,
            resource_id: self.resource_id.clone(),
            user_id: self.user_id.clone(),
        })
    }
    fn page_info(&self) -> PageInfo {
        PageInfo {
            cursor: self.cursor,
            page_size: self.page_size,
            desc: self.desc,
        }
    }
}

impl ToSql for ReservationFilter {
    fn to_sql(&self) -> String {
        let middle_plus = i64::from(self.cursor.is_some());
        let limit = self.page_size + 1 + middle_plus;

        let status = self.get_status();

        let cursor_cond = if self.desc {
            format!("id < {}", self.get_cursor())
        } else {
            format!("id > {}", self.get_cursor())
        };

        let user_cursor_cond = match (self.user_id.is_empty(), self.resource_id.is_empty()) {
            (true, true) => "TRUE".into(),
            (true, false) => format!("resource_id = '{}'", self.resource_id),
            (false, true) => format!("user_id = '{}'", self.user_id),
            (false, false) => format!(
                "user_id = '{}' AND resource_id = '{}'",
                self.user_id, self.resource_id
            ),
        };

        let direction = if self.desc { "DESC" } else { "ASC" };

        format!(
            "SELECT * FROM rsvp.reservations WHERE status = '{}'::rsvp.reservation_status AND {} AND {} ORDER BY id {} LIMIT {}",
            status, cursor_cond, user_cursor_cond, direction, limit
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_should_generate_correct_page() {}
    #[test]
    fn filter_should_generate_correct_sql() {
        let filter = ReservationFilterBuilder::default()
            .user_id("tyrchen")
            .build()
            .unwrap();

        let sql = filter.to_sql();

        assert_eq!(sql, "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id > 0 AND user_id = 'tyrchen' ORDER BY id ASC LIMIT 11")
    }
}
