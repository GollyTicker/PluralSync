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
    type Out = UserConfigDbEntries<secrets::Encrypted>;
    log::debug!("# | db::get_user | {user_id}");
    sqlx::query_as!(
        Out,
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
            enable_from_pluralkit,
            enable_from_sp,
            privacy_fine_grained AS \"privacy_fine_grained: users::PrivacyFineGrained\",
            privacy_fine_grained_buckets,
            history_limit,
            history_truncate_after_days,
            fronter_channel_wait_increment,
            discord_rich_presence_url AS \"discord_rich_presence_url: users::DiscordRichPresenceUrl\",
            discord_rich_presence_url_custom,
            from_pluralkit_prefer_displayname,
            from_pluralkit_respect_member_visibility,
            from_pluralkit_respect_field_visibility,
            '' AS \"simply_plural_token: secrets::Encrypted\",
            '' AS \"discord_status_message_token: secrets::Encrypted\",
            '' AS \"vrchat_username: secrets::Encrypted\",
            '' AS \"vrchat_password: secrets::Encrypted\",
            '' AS \"vrchat_cookie: secrets::Encrypted\",
            '' AS \"pluralkit_token: secrets::Encrypted\",
            '' AS \"from_pluralkit_webhook_signing_token: secrets::Encrypted\",
            false AS \"valid_constraints: constraints::InvalidConstraints\"
            FROM users WHERE id = $1",
        user_id.inner
    )
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
    application_user_secrets: &secrets::ApplicationUserSecrets,
) -> Result<users::UserConfigForUpdater> {
    log::debug!("# | db::get_user_config_with_secrets | {user_id}");

    let config = get_user_secrets(db_pool, user_id, application_user_secrets).await?;

    let (config, _) = users::create_config_with_strong_constraints(user_id, client, &config)?;

    Ok(config)
}

pub async fn get_user_secrets(
    db_pool: &PgPool,
    user_id: &UserId,
    application_user_secret: &secrets::ApplicationUserSecrets,
) -> Result<UserConfigDbEntries<secrets::Decrypted, constraints::ValidConstraints>> {
    type Out = UserConfigDbEntries<secrets::Decrypted, constraints::ValidConstraints>;

    log::debug!("# | db::get_user_secrets | {user_id}");
    let secrets_key = compute_user_secrets_key(user_id, application_user_secret);
    sqlx::query_as!(
        Out,
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
            enable_from_pluralkit,
            enable_from_sp,
            privacy_fine_grained AS \"privacy_fine_grained: crate::users::PrivacyFineGrained\",
            privacy_fine_grained_buckets,
            history_limit,
            history_truncate_after_days,
            fronter_channel_wait_increment,
            discord_rich_presence_url AS \"discord_rich_presence_url: crate::users::DiscordRichPresenceUrl\",
            discord_rich_presence_url_custom,
            pgp_sym_decrypt(enc__simply_plural_token, $2) AS \"simply_plural_token: secrets::Decrypted\",
            pgp_sym_decrypt(enc__discord_status_message_token, $2) AS \"discord_status_message_token: secrets::Decrypted\",
            pgp_sym_decrypt(enc__vrchat_username, $2) AS \"vrchat_username: secrets::Decrypted\",
            pgp_sym_decrypt(enc__vrchat_password, $2) AS \"vrchat_password: secrets::Decrypted\",
            pgp_sym_decrypt(enc__vrchat_cookie, $2) AS \"vrchat_cookie: secrets::Decrypted\",
            pgp_sym_decrypt(enc__pluralkit_token, $2) AS \"pluralkit_token: secrets::Decrypted\",
            pgp_sym_decrypt(enc__from_pluralkit_webhook_signing_token, $2) AS \"from_pluralkit_webhook_signing_token: secrets::Decrypted\",
            from_pluralkit_prefer_displayname,
            from_pluralkit_respect_member_visibility,
            from_pluralkit_respect_field_visibility,
            true AS \"valid_constraints: constraints::ValidConstraints\"
            FROM users WHERE id = $1",
        user_id.inner,
        secrets_key.inner
    )
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

    // Note: Using query_as() instead of query!() because custom enums need Encode trait
    // which is complex to implement. UPDATE queries don't benefit as much from compile-time
    // verification since they don't return data.
    let _: Option<()> = sqlx::query_as(
        "UPDATE users
        SET
            website_system_name = $2,
            status_prefix = $3,
            status_no_fronts = $4,
            status_truncate_names_to = $5,
            enable_discord_status_message = $6,
            enable_vrchat = $7,
            enc__simply_plural_token = pgp_sym_encrypt($9, $8),
            enc__discord_status_message_token = pgp_sym_encrypt($10, $8),
            enc__vrchat_username = pgp_sym_encrypt($11, $8),
            enc__vrchat_password = pgp_sym_encrypt($12, $8),
            enc__vrchat_cookie = pgp_sym_encrypt($13, $8),
            enable_discord = $14,
            enable_website = $15,
            website_url_name = $16,
            show_members_non_archived = $17,
            show_members_archived = $18,
            show_custom_fronts = $19,
            respect_front_notifications_disabled = $20,
            privacy_fine_grained = $21,
            privacy_fine_grained_buckets = $22,
            enable_to_pluralkit = $23,
            enc__pluralkit_token = pgp_sym_encrypt($24, $8),
            history_limit = $25,
            history_truncate_after_days = $26,
            fronter_channel_wait_increment = $27,
            enable_from_pluralkit = $28,
            enc__from_pluralkit_webhook_signing_token = pgp_sym_encrypt($29, $8),
            enable_from_sp = $30,
            discord_rich_presence_url = $31,
            discord_rich_presence_url_custom = $32,
            from_pluralkit_prefer_displayname = $33,
            from_pluralkit_respect_member_visibility = $34,
            from_pluralkit_respect_field_visibility = $35
        WHERE id = $1",
    )
    .bind(user_id.inner)
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
    .bind(&config.privacy_fine_grained_buckets)
    .bind(config.enable_to_pluralkit)
    .bind(config.pluralkit_token.map(|s| s.secret))
    .bind(config.history_limit)
    .bind(config.history_truncate_after_days)
    .bind(config.fronter_channel_wait_increment)
    .bind(config.enable_from_pluralkit)
    .bind(
        config
            .from_pluralkit_webhook_signing_token
            .map(|s| s.secret),
    )
    .bind(config.enable_from_sp)
    .bind(&config.discord_rich_presence_url)
    .bind(&config.discord_rich_presence_url_custom)
    .bind(config.from_pluralkit_prefer_displayname)
    .bind(config.from_pluralkit_respect_member_visibility)
    .bind(config.from_pluralkit_respect_field_visibility)
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

    let hex_string = hex::encode(digest);

    secrets::UserSecretsDecryptionKey { inner: hex_string }
}

#[cfg(test)]
mod tests {
    use crate::{
        database::{ApplicationUserSecrets, user_config_queries::compute_user_secrets_key},
        users::UserId,
    };

    #[test]
    fn correct_user_secrets_key() {
        let user_id: UserId = UserId {
            inner: uuid::uuid!("33c89b20-be5b-4f8e-adfa-0d444719a6db"),
        };
        let app_user_secret = ApplicationUserSecrets {
            inner: "some-secret".to_owned(),
        };

        assert_eq!(
            "6804f20948f345a84a8eb8229d785b967f1bc6fbd4c29f3b1ec02c101b1edde2",
            compute_user_secrets_key(&user_id, &app_user_secret).inner
        );
    }
}
