use anyhow::{Result, anyhow};
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

use pluralsync_base::users::{JwtString, Secret, UserProvidedPassword};
use rand::{Rng, distr, prelude::ThreadRng};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{database, users::jwt};

#[derive(Debug, Serialize, Deserialize, FromRow, sqlx::Type)]
pub struct SecretHashString {
    pub inner: String,
}
impl From<String> for SecretHashString {
    fn from(val: String) -> Self {
        Self { inner: val }
    }
}

pub fn generate_secret() -> Secret {
    let secret = ThreadRng::default()
        .sample_iter(distr::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    Secret { inner: secret }
}

pub fn create_secret_hash(secret: &UserProvidedPassword) -> Result<SecretHashString> {
    // don't allow external user to infer what exactly failed
    let salt = SaltString::generate(&mut OsRng);

    let pwh = Argon2::default()
        .hash_password(secret.inner.inner.as_bytes(), &salt)
        .map_err(|_| anyhow!("Registration failed"))?;

    Ok(SecretHashString {
        inner: pwh.to_string(),
    })
}

pub fn verify_password_and_create_token(
    password: &UserProvidedPassword,
    user_info: &database::UserInfo,
    jwt_secret: &jwt::ApplicationJwtSecret,
) -> Result<JwtString> {
    // don't allow external user to infer what exactly failed

    let pwh = PasswordHash::new(&user_info.password_hash.inner)
        .map_err(|_| anyhow!("Invalid email/password"))?;

    Argon2::default()
        .verify_password(password.inner.inner.as_bytes(), &pwh)
        .map_err(|_| anyhow!("Invalid email/password"))?;

    let token = jwt::create_token(&user_info.id, jwt_secret)
        .map_err(|_| anyhow!("Invalid email/password"))?;

    Ok(token)
}
