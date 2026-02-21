use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{FromRow, types::Uuid};

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, sqlx::Type, Eq, Hash, PartialEq, Type)]
pub struct UserId {
    #[specta(type = String)]
    pub inner: Uuid,
}

impl From<Uuid> for UserId {
    fn from(val: Uuid) -> Self {
        Self { inner: val }
    }
}

impl TryFrom<&str> for UserId {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let uuid = Uuid::from_str(value)?;
        Ok(Self { inner: uuid })
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserId({})", self.inner)
    }
}
