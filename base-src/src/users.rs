use std::fmt::Display;

use serde::{Deserialize, Serialize};
use specta;

#[derive(
    Debug, Serialize, Deserialize, Clone, sqlx::FromRow, sqlx::Type, specta::Type, PartialEq, Eq,
)]
pub struct Email {
    pub inner: String,
}

impl From<String> for Email {
    fn from(val: String) -> Self {
        Self { inner: val }
    }
}

impl Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Email({})", self.inner)
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct UserLoginCredentials {
    pub email: Email,
    pub password: UserProvidedPassword,
    pub client_version: String,
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

impl Display for JwtString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.inner.chars().take(5).collect();
        write!(f, "JwtString({s}...)")
    }
}
