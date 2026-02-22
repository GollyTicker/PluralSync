use anyhow::{Result, anyhow};
use pluralsync_base::users::Email;
use sqlx::{FromRow, PgPool};

use crate::users::{SecretHashString, UserId};

// ============================================================================
// Temporary User & Email Verification
// ============================================================================

pub async fn find_temporary_user_by_token_hash(
    db_pool: &PgPool,
    token_hash: &SecretHashString,
) -> Result<Option<TemporaryUser>> {
    log::debug!("# | db::find_temporary_user_by_token_hash | {token_hash}");
    let temporary_user = sqlx::query_as!(
        TemporaryUser,
        "SELECT
            id,
            email,
            password_hash,
            email_verification_token_hash,
            email_verification_token_expires_at,
            created_at
        FROM temporary_users
        WHERE email_verification_token_hash = $1",
        token_hash.inner
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    Ok(temporary_user)
}

pub async fn create_or_update_temporary_user(
    db_pool: &PgPool,
    email: Email,
    password_hash: SecretHashString,
    email_verification_token_hash: SecretHashString,
    email_verification_token_expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    log::debug!("# | db::create_or_update_temporary_user | {email}");
    sqlx::query!(
        "INSERT INTO temporary_users (email, password_hash, email_verification_token_hash, email_verification_token_expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (email) DO UPDATE
        SET
            password_hash = EXCLUDED.password_hash,
            email_verification_token_hash = EXCLUDED.email_verification_token_hash,
            email_verification_token_expires_at = EXCLUDED.email_verification_token_expires_at,
            created_at = NOW()",
        email.inner,
        password_hash.inner,
        email_verification_token_hash.inner,
        email_verification_token_expires_at
    )
    .execute(db_pool)
    .await
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}

pub async fn find_user_id_by_email_verification_token_hash(
    db_pool: &PgPool,
    token_hash: &SecretHashString,
) -> Result<Option<UserId>> {
    log::debug!("# | db::find_user_id_by_email_verification_token_hash | {token_hash}");
    let user_id = sqlx::query_as!(
        UserId,
        "SELECT
            id AS inner
        FROM users WHERE email_verification_token_hash = $1",
        token_hash.inner
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    Ok(user_id)
}

// ============================================================================
// Helper Structs
// ============================================================================

#[derive(FromRow)]
pub struct TemporaryUser {
    pub id: UserId,
    pub email: Email,
    pub password_hash: SecretHashString,
    pub email_verification_token_hash: SecretHashString,
    pub email_verification_token_expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
