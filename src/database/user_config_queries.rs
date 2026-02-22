use anyhow::Result;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::{
    database::{Decrypted, constraints, secrets},
    setup,
    users::{self, UserConfigDbEntries, UserId},
};

// ============================================================================
// User Config (Non-Secret)
// ============================================================================

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
            history_limit,
            history_truncate_after_days,
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
    .map_err(|e| anyhow::anyhow!(e))
}

// ============================================================================
// User Config with Secrets
// ============================================================================

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
            history_limit,
            history_truncate_after_days,
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
    .map_err(|e| anyhow::anyhow!(e))
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
            enc__pluralkit_token = pgp_sym_encrypt($25, $9),
            history_limit = $26,
            history_truncate_after_days = $27
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
    .bind(config.history_limit)
    .bind(config.history_truncate_after_days)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

pub async fn modify_user_secrets(
    db_pool: &PgPool,
    user_id: &UserId,
    application_user_secrets: &secrets::ApplicationUserSecrets,
    modify: impl FnOnce(&mut UserConfigDbEntries<Decrypted, constraints::ValidConstraints>),
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

// ============================================================================
// Helper Functions
// ============================================================================

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
