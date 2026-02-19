use anyhow::{Result, anyhow};
use pluralsync_base::users::Email;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use std::option::Option;

use crate::{
    database::{Decrypted, ValidConstraints, constraints, secrets},
    metrics::PASSWORD_RESET_REQUESTS_TOTAL,
    setup,
    users::{self, SecretHashString, UserConfigDbEntries, UserId},
};

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

pub async fn create_password_reset_request(
    db_pool: &PgPool,
    user_id: &UserId,
    token_hash: &SecretHashString,
    expires_at: &chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    log::debug!("# | db::create_password_reset_request | {user_id}");
    PASSWORD_RESET_REQUESTS_TOTAL
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

pub async fn get_user(
    db_pool: &PgPool,
    user_id: &UserId,
) -> Result<UserConfigDbEntries<secrets::Encrypted>> {
    log::debug!("# | db::get_user | {user_id}");
    sqlx::query_as(
        "SELECT
            website_system_name,
            website_url_name,
            status_prefix,
            status_no_fronts,
            status_truncate_names_to,
            show_members_non_archived,
            show_members_archived,
            show_custom_fronts,
            respect_front_notifications_disabled,
            enable_discord,
            enable_discord_status_message,
            enable_vrchat,
            enable_website,
            enable_to_pluralkit,
            privacy_fine_grained,
            privacy_fine_grained_buckets,
            '' AS simply_plural_token,
            '' AS discord_status_message_token,
            '' AS vrchat_username,
            '' AS vrchat_password,
            '' AS vrchat_cookie,
            '' AS pluralkit_token,
            false AS valid_constraints
            FROM users WHERE id = $1",
    )
    .bind(user_id.inner)
    .fetch_one(db_pool)
    .await
    .map_err(|e| anyhow!(e))
}

pub async fn set_user_config_secrets(
    db_pool: &PgPool,
    user_id: &UserId,
    config: UserConfigDbEntries<secrets::Decrypted, constraints::ValidConstraints>,
    application_user_secret: &secrets::ApplicationUserSecrets,
) -> Result<()> {
    log::debug!("# | db::set_user_config_secrets | {user_id}");

    let secrets_key = compute_user_secrets_key(user_id, application_user_secret);

    let _: Option<UserConfigDbEntries<secrets::Decrypted>> = sqlx::query_as(
        "UPDATE users
        SET
            website_system_name = $3,
            status_prefix = $4,
            status_no_fronts = $5,
            status_truncate_names_to = $6,
            enable_discord_status_message = $7,
            enable_vrchat = $8,
            enc__simply_plural_token = pgp_sym_encrypt($10, $9),
            enc__discord_status_message_token = pgp_sym_encrypt($11, $9),
            enc__vrchat_username = pgp_sym_encrypt($12, $9),
            enc__vrchat_password = pgp_sym_encrypt($13, $9),
            enc__vrchat_cookie = pgp_sym_encrypt($14, $9),
            enable_discord = $15,
            enable_website = $16,
            website_url_name = $17,
            show_members_non_archived = $18,
            show_members_archived = $19,
            show_custom_fronts = $20,
            respect_front_notifications_disabled = $21,
            privacy_fine_grained = $22,
            privacy_fine_grained_buckets = $23,
            enable_to_pluralkit = $24,
            enc__pluralkit_token = pgp_sym_encrypt($25, $9)
        WHERE id = $1",
    )
    .bind(user_id.inner)
    .bind(0)
    .bind(&config.website_system_name)
    .bind(&config.status_prefix)
    .bind(&config.status_no_fronts)
    .bind(config.status_truncate_names_to)
    .bind(config.enable_discord_status_message)
    .bind(config.enable_vrchat)
    .bind(&secrets_key.inner)
    .bind(config.simply_plural_token.map(|s| s.secret))
    .bind(config.discord_status_message_token.map(|s| s.secret))
    .bind(config.vrchat_username.map(|s| s.secret))
    .bind(config.vrchat_password.map(|s| s.secret))
    .bind(config.vrchat_cookie.map(|s| s.secret))
    .bind(config.enable_discord)
    .bind(config.enable_website)
    .bind(config.website_url_name)
    .bind(config.show_members_non_archived)
    .bind(config.show_members_archived)
    .bind(config.show_custom_fronts)
    .bind(config.respect_front_notifications_disabled)
    .bind(config.privacy_fine_grained)
    .bind(config.privacy_fine_grained_buckets)
    .bind(config.enable_to_pluralkit)
    .bind(config.pluralkit_token.map(|s| s.secret))
    .fetch_optional(db_pool)
    .await
    .map_err(|e| anyhow!(e))?;

    Ok(())
}

pub async fn get_user_config_with_secrets(
    db_pool: &PgPool,
    user_id: &UserId,
    client: &reqwest::Client,
    application_user_secret: &secrets::ApplicationUserSecrets,
) -> Result<users::UserConfigForUpdater> {
    log::debug!("# | db::get_user_config_with_secrets | {user_id}");

    let config = get_user_secrets(db_pool, user_id, application_user_secret).await?;

    let (config, _) = users::create_config_with_strong_constraints(user_id, client, &config)?;

    Ok(config)
}

pub async fn get_user_secrets(
    db_pool: &PgPool,
    user_id: &UserId,
    application_user_secret: &secrets::ApplicationUserSecrets,
) -> Result<UserConfigDbEntries<secrets::Decrypted, constraints::ValidConstraints>> {
    log::debug!("# | db::get_user_secrets | {user_id}");

    let secrets_key = compute_user_secrets_key(user_id, application_user_secret);

    sqlx::query_as(
        "SELECT
            website_system_name,
            website_url_name,
            status_prefix,
            status_no_fronts,
            status_truncate_names_to,
            show_members_non_archived,
            show_members_archived,
            show_custom_fronts,
            respect_front_notifications_disabled,
            enable_website,
            enable_discord,
            enable_discord_status_message,
            enable_vrchat,
            enable_to_pluralkit,
            privacy_fine_grained,
            privacy_fine_grained_buckets,
            pgp_sym_decrypt(enc__simply_plural_token, $2) AS simply_plural_token,
            pgp_sym_decrypt(enc__discord_status_message_token, $2) AS discord_status_message_token,
            pgp_sym_decrypt(enc__vrchat_username, $2) AS vrchat_username,
            pgp_sym_decrypt(enc__vrchat_password, $2) AS vrchat_password,
            pgp_sym_decrypt(enc__vrchat_cookie, $2) AS vrchat_cookie,
            pgp_sym_decrypt(enc__pluralkit_token, $2) AS pluralkit_token,
            true AS valid_constraints
            FROM users WHERE id = $1",
    )
    .bind(user_id.inner)
    .bind(secrets_key.inner)
    .fetch_one(db_pool)
    .await
    .map_err(|e| anyhow!(e))
}

pub async fn modify_user_secrets(
    db_pool: &PgPool,
    user_id: &UserId,
    application_user_secrets: &secrets::ApplicationUserSecrets,
    modify: impl FnOnce(&mut UserConfigDbEntries<Decrypted, ValidConstraints>),
) -> Result<()> {
    log::debug!("# | db::modify_user_secrets | {user_id}");

    let mut user_with_secrets =
        get_user_secrets(db_pool, user_id, application_user_secrets).await?;

    modify(&mut user_with_secrets);

    let unused_client = setup::make_client()?;

    let (_, new_config) =
        users::create_config_with_strong_constraints(user_id, &unused_client, &user_with_secrets)?;

    let () =
        set_user_config_secrets(db_pool, user_id, new_config, application_user_secrets).await?;

    log::debug!("# | db::modify_user_secrets | {user_id} | modified");

    Ok(())
}

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

fn compute_user_secrets_key(
    user_id: &UserId,
    application_user_secret: &secrets::ApplicationUserSecrets,
) -> secrets::UserSecretsDecryptionKey {
    let user_id = user_id.inner.to_string();
    let app_user_secret = &application_user_secret.inner;

    let digest = {
        let mut hasher = Sha256::new();
        hasher.update(user_id);
        hasher.update(app_user_secret);
        hasher.finalize()
    };

    let hex_string = format!("{digest:x}");

    secrets::UserSecretsDecryptionKey { inner: hex_string }
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

#[derive(FromRow)]
pub struct TemporaryUser {
    pub id: UserId,
    pub email: Email,
    pub password_hash: SecretHashString,
    pub email_verification_token_hash: SecretHashString,
    pub email_verification_token_expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
