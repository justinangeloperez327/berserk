use crate::{DatabaseError, ErrorKind, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page<T> {
    items: Vec<T>,
    page: u64,
    per_page: u64,
    total: u64,
    last_page: u64,
}

impl<T> Page<T> {
    pub(crate) fn validate(page: u64, per_page: u64) -> Result<()> {
        if page == 0 {
            return Err(error("page numbers start at 1"));
        }
        if per_page == 0 {
            return Err(error("per_page must be greater than zero"));
        }
        if per_page > 1_000 {
            return Err(error("per_page cannot exceed 1000"));
        }
        Ok(())
    }

    pub(crate) fn new(items: Vec<T>, page: u64, per_page: u64, total: u64) -> Result<Self> {
        Self::validate(page, per_page)?;
        let last_page = if total == 0 {
            1
        } else {
            1 + (total - 1) / per_page
        };
        Ok(Self {
            items,
            page,
            per_page,
            total,
            last_page,
        })
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }
    pub fn into_items(self) -> Vec<T> {
        self.items
    }
    pub const fn page(&self) -> u64 {
        self.page
    }
    pub const fn per_page(&self) -> u64 {
        self.per_page
    }
    pub const fn total(&self) -> u64 {
        self.total
    }
    pub const fn last_page(&self) -> u64 {
        self.last_page
    }
    pub const fn has_previous(&self) -> bool {
        self.page > 1
    }
    pub const fn has_next(&self) -> bool {
        self.page < self.last_page
    }
}

fn error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(ErrorKind::Query, message)
}
