use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{self, Decode, FromRow, Postgres, error::BoxDynError, postgres};

#[derive(Clone)]
pub struct UserSecretsDecryptionKey {
    pub inner: String,
}

#[derive(Clone)]
pub struct ApplicationUserSecrets {
    pub inner: String,
}

pub trait SecretType: Default + Clone {}

impl SecretType for Encrypted {}
impl SecretType for Decrypted {}

#[derive(Default, Clone, Serialize, FromRow, PartialEq, Eq, fmt::Debug)]
#[sqlx(type_name = "encrypted")]
pub struct Encrypted {}

impl From<String> for Encrypted {
    fn from(_: String) -> Self {
        Self {}
    }
}

// manual implementation of `Type<Postgres>` because derive doesn't work for non-newtype structs
impl sqlx::Type<Postgres> for Encrypted {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <String as sqlx::Type<Postgres>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Postgres> for Encrypted {
    fn decode(value: postgres::PgValueRef<'r>) -> Result<Self, BoxDynError> {
        <String as Decode<Postgres>>::decode(value)?;
        Ok(Self {})
    }
}

#[derive(Default, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, specta::Type)]
#[sqlx(type_name = "decrypted")]
pub struct Decrypted {
    pub secret: String,
}

// manual implementation of `Type<Postgres>` because derive doesn't work for non-newtype structs
impl sqlx::Type<Postgres> for Decrypted {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <String as sqlx::Type<Postgres>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Postgres> for Decrypted {
    fn decode(value: postgres::PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let secret = <String as Decode<Postgres>>::decode(value)?;
        Ok(Self { secret })
    }
}

impl From<&str> for Decrypted {
    fn from(value: &str) -> Self {
        Self {
            secret: value.to_string(),
        }
    }
}

impl From<String> for Decrypted {
    fn from(secret: String) -> Self {
        Self { secret }
    }
}
