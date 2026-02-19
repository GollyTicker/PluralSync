use std::fmt::Display;

use anyhow::{Result, anyhow};
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};

use pluralsync_base::users::{JwtString, Secret, UserProvidedPassword};
use rand::{RngExt as _, distr, prelude::ThreadRng};
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

pub struct SecretHashOptions {
    pub use_specific_salt: Option<String>,
}

impl Display for SecretHashString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.inner.chars().take(3).collect();
        write!(f, "SecretHashString({s}...)")
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

pub fn create_secret_hash(secret: &Secret, options: SecretHashOptions) -> Result<SecretHashString> {
    // don't allow external user to infer what exactly failed
    let salt = options
        .use_specific_salt
        .map(|raw_salt_str| {
            let encoded_salt = BASE64_STANDARD_NO_PAD.encode(raw_salt_str.as_bytes());

            if encoded_salt.len() < 22 {
                return Err(anyhow!("Provided raw salt string is too short; base64 encoded length is less than 22 characters"));
            }

            SaltString::from_b64(encoded_salt.as_str())
                .inspect_err(|e| log::warn!("base64 encoded salt is not valid or has invalid length: {e}"))
                .map_err(|_| anyhow!("Failed to create SaltString from encoded salt."))
        })
        .transpose()?
        .unwrap_or_else(|| SaltString::generate(&mut OsRng));

    let pwh = Argon2::default()
        .hash_password(secret.inner.as_bytes(), &salt)
        .map_err(|_| anyhow!("secret hashing failed"))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let secret = generate_secret();
        assert_eq!(secret.inner.len(), 32);
        assert!(secret.inner.chars().all(|c| c.is_alphanumeric()));

        let secret2 = generate_secret();
        assert_ne!(secret.inner, secret2.inner);
    }

    #[test]
    fn test_create_secret_hash() -> Result<()> {
        let secret = Secret {
            inner: "my_test_secret".to_string(),
        };
        let hash1 = create_secret_hash(
            &secret,
            SecretHashOptions {
                use_specific_salt: Some("some_rand_16_bytes".to_owned()),
            },
        )?;
        assert!(!hash1.inner.is_empty());
        assert!(hash1.inner.starts_with("$argon2id$"));

        let hash2 = create_secret_hash(
            &secret,
            SecretHashOptions {
                use_specific_salt: Some("some_rand_16_bytes".to_owned()),
            },
        )?;
        assert_eq!(
            hash1.inner, hash2.inner,
            "Hashes for the same secret and same raw salt should be equal"
        );

        let hash3 = create_secret_hash(
            &secret,
            SecretHashOptions {
                use_specific_salt: Some("another_16_bytes".to_owned()),
            },
        )?;
        assert_ne!(
            hash1.inner, hash3.inner,
            "Hashes for the same secret should be different when given different raw salts"
        );

        let hash4 = create_secret_hash(
            &Secret {
                inner: "different-secret".to_string(),
            },
            SecretHashOptions {
                use_specific_salt: Some("some_rand_16_bytes".to_owned()),
            },
        )?;
        assert_ne!(
            hash1.inner, hash4.inner,
            "Hashes for a different secret should be different even with the same raw salt"
        );

        Ok(())
    }
}
