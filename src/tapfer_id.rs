use crate::error::TapferError;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct TapferId {
    inner: Uuid,
}

impl TapferId {
    pub fn new_random() -> Self {
        Self {
            inner: Uuid::new_v4(),
        }
    }
    pub fn from_id(id: Uuid) -> Self {
        Self { inner: id }
    }
}

impl FromStr for TapferId {
    type Err = TapferError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            inner: Uuid::parse_str(s)?,
        })
    }
}

impl Display for TapferId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
