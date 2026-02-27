use std::fmt;

use derive_more;
use serde::{Deserialize, Serialize};
use specta;

#[derive(
    derive_more::Display,
    Serialize,
    Deserialize,
    Clone,
    sqlx::FromRow,
    sqlx::Type,
    specta::Type,
    PartialEq,
    Eq,
)]
pub struct Email {
    pub inner: String,
}

impl From<String> for Email {
    fn from(val: String) -> Self {
        Self { inner: val }
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct UserLoginCredentials {
    pub email: Email,
    pub password: UserProvidedPassword,
}

impl UserLoginCredentials {
    #[must_use]
    pub const fn is_empty_and_thus_invalid(&self) -> bool {
        self.email.inner.is_empty() || self.password.inner.inner.is_empty()
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct UserProvidedPassword {
    pub inner: Secret,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct PasswordResetToken {
    pub inner: Secret,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct EmailVerificationToken {
    pub inner: Secret,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct Secret {
    pub inner: String,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct JwtString {
    pub inner: String,
}

impl fmt::Display for JwtString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.inner.chars().take(5).collect();
        write!(f, "JwtString({s}...)")
    }
}
