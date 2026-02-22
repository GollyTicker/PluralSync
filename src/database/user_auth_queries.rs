use anyhow::{Result, anyhow};
use pluralsync_base::users::Email;
use sqlx::{FromRow, PgPool};

use crate::{
    metrics,
    users::{SecretHashString, UserId},
};

// ============================================================================
// User Creation & Authentication
// ============================================================================

pub async fn create_user(
    db_pool: &PgPool,
    email: Email,
    password_hash: SecretHashString,
) -> Result<()> {
    log::debug!("# | db::create_user | {email}");
    sqlx::query!(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2)",
        email.inner,
        password_hash.inner
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

pub async fn get_user_id(db_pool: &PgPool, email: Email) -> Result<UserId> {
    log::debug!("# | db::get_user_id | {email}");
    sqlx::query_as!(
        UserId,
        "SELECT
            id AS inner
        FROM users WHERE email = $1",
        email.inner
    )
    .fetch_one(db_pool)
    .await
    .map_err(|e| anyhow!(e))
}

pub async fn get_user_info(db_pool: &PgPool, user_id: UserId) -> Result<UserInfo> {
    log::debug!("# | db::get_user_info | {user_id}");

    let row = sqlx::query_as::<_, UserInfoRaw>(
        "SELECT
            id,
            email,
            password_hash,
            created_at,
            new_email,
            email_verification_token_hash,
            email_verification_token_expires_at
        FROM users WHERE id = $1",
    )
    .bind(user_id.inner)
    .fetch_one(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    Ok(row.into_user_info())
}

pub async fn find_user_by_website_url_name(
    db_pool: &PgPool,
    website_url_name: &str,
) -> Result<UserInfo> {
    log::debug!("# | db::find_user_by_website_url_name | {website_url_name}");
    let row = sqlx::query_as::<_, UserInfoRaw>(
        "SELECT
            id,
            email,
            password_hash,
            created_at,
            new_email,
            email_verification_token_hash,
            email_verification_token_expires_at
            FROM users WHERE website_url_name = $1",
    )
    .bind(website_url_name)
    .fetch_one(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    Ok(row.into_user_info())
}

pub async fn delete_user(db_pool: &PgPool, user_id: &UserId) -> Result<()> {
    log::debug!("# | db::delete_user | {user_id}");
    sqlx::query!(
        "DELETE FROM users
        WHERE id = $1",
        user_id.inner
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

// ============================================================================
// Password Reset
// ============================================================================

pub async fn create_password_reset_request(
    db_pool: &PgPool,
    user_id: &UserId,
    token_hash: &SecretHashString,
    expires_at: &chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    log::debug!("# | db::create_password_reset_request | {user_id}");
    metrics::PASSWORD_RESET_REQUESTS_TOTAL
        .with_label_values(&["create_password_reset_request"])
        .inc();
    // remove any previous password reset attempts
    sqlx::query!(
        "DELETE FROM password_reset_requests
        WHERE user_id = $1",
        user_id.inner
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))?;
    sqlx::query!(
        "INSERT INTO
        password_reset_requests (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)",
        user_id.inner,
        token_hash.inner,
        expires_at
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

pub async fn verify_password_reset_request_matches(
    db_pool: &PgPool,
    token_hash: &SecretHashString,
) -> Result<UserId> {
    log::debug!("# | db::verify_reset_token | {token_hash}");
    sqlx::query_as!(
        UserId,
        "SELECT user_id as inner
        FROM password_reset_requests
        WHERE token_hash = $1
            AND expires_at > NOW()",
        token_hash.inner
    )
    .fetch_one(db_pool)
    .await
    .map_err(|e| anyhow!(e))
}

pub async fn delete_password_reset_request(db_pool: &PgPool, user_id: &UserId) -> Result<()> {
    log::debug!("# | db::delete_password_reset_request | {user_id}");
    sqlx::query!(
        "DELETE FROM password_reset_requests
        WHERE user_id = $1",
        user_id.inner
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

pub async fn update_user_password(
    db_pool: &PgPool,
    user_id: &UserId,
    password_hash: &SecretHashString,
) -> Result<()> {
    log::debug!("# | db::update_user_password | {user_id}");
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        password_hash.inner,
        user_id.inner
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

// ============================================================================
// Email Change
// ============================================================================

pub async fn update_user_email_change_fields(
    db_pool: &PgPool,
    user_id: &UserId,
    new_email: Email,
    email_verification_token_hash: SecretHashString,
    email_verification_token_expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    log::debug!("# | db::update_user_email_change_fields | {user_id} for new email {new_email}");
    sqlx::query!(
        "UPDATE users
        SET
            new_email = $1,
            email_verification_token_hash = $2,
            email_verification_token_expires_at = $3
        WHERE id = $4",
        new_email.inner,
        email_verification_token_hash.inner,
        email_verification_token_expires_at,
        user_id.inner
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

pub async fn set_new_verified_email(
    db_pool: &PgPool,
    user_id: &UserId,
    new_email: Email,
) -> Result<()> {
    log::debug!("# | db::update_user_email | {user_id} to {new_email}");
    sqlx::query!(
        "UPDATE users SET email = $1, new_email = NULL WHERE id = $2",
        new_email.inner,
        user_id.inner
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

// ============================================================================
// All Users
// ============================================================================

pub async fn get_all_users(db_pool: &PgPool) -> Result<Vec<UserId>> {
    log::debug!("# | db::get_all_users");

    let users = sqlx::query_as!(
        UserId,
        "SELECT
            id AS inner
        FROM users"
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    log::debug!("# | db::get_all_users | retrieved={}", users.len());

    Ok(users)
}

// ============================================================================
// Helper Structs
// ============================================================================

#[derive(FromRow)]
struct UserInfoRaw {
    id: sqlx::types::Uuid,
    email: String,
    password_hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
    new_email: Option<String>,
    email_verification_token_hash: Option<String>,
    email_verification_token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl UserInfoRaw {
    fn into_user_info(self) -> UserInfo {
        UserInfo {
            id: UserId::from(self.id),
            email: Email::from(self.email),
            password_hash: SecretHashString {
                inner: self.password_hash,
            },
            created_at: self.created_at,
            new_email: self.new_email.map(Email::from),
            email_verification_token_hash: self.email_verification_token_hash,
            email_verification_token_expires_at: self.email_verification_token_expires_at,
        }
    }
}

#[derive(FromRow)]
pub struct UserInfo {
    pub id: UserId,
    pub email: Email,
    pub password_hash: SecretHashString,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub new_email: Option<Email>,
    pub email_verification_token_hash: Option<String>,
    pub email_verification_token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}
