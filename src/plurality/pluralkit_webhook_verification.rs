use anyhow::Result;
use sqlx::PgPool;

use crate::{
    database, plurality, setup,
    users::{self, UserId},
};

/// Daily cron job: verify all `PluralKit` webhooks are still active.
/// If a webhook is deactivated, sends an email notification and disables the webhook.
pub async fn verify_pluralkit_webhooks(
    db_pool: PgPool,
    client: reqwest::Client,
    application_user_secrets: database::ApplicationUserSecrets,
    smtp_config: setup::SmtpConfig,
) -> Result<()> {
    log::info!("# | verify_pluralkit_webhooks | Starting daily verification");

    let users = database::get_users_with_pluralkit_webhook_enabled(&db_pool).await?;

    log::info!(
        "# | verify_pluralkit_webhooks | Found {} users to verify",
        users.len()
    );

    for user in users {
        if let Err(e) = verify_single_user(
            &db_pool,
            &client,
            &application_user_secrets,
            &smtp_config,
            &user,
        )
        .await
        {
            log::warn!("Failed to verify webhook for user {user}: {e}");
        }
    }

    log::info!("# | verify_pluralkit_webhooks | Verification complete");

    Ok(())
}

async fn verify_single_user(
    db_pool: &PgPool,
    client: &reqwest::Client,
    application_user_secrets: &database::ApplicationUserSecrets,
    smtp_config: &setup::SmtpConfig,
    user: &UserId,
) -> Result<()> {
    let config =
        database::get_user_config_with_secrets(db_pool, user, client, application_user_secrets)
            .await?;

    let expected_webhook_url = Some(format!(
        "{}/api/webhook/pluralkit/{}",
        smtp_config.frontend_base_url, user.inner
    ));

    let system = plurality::http_pluralkit_system(client, &config.pluralkit_token, user).await?;

    let webhook_valid = system.webhook_url == expected_webhook_url;

    if webhook_valid {
        log::debug!("Webhook OK for {user}");
        return Ok(());
    }

    log::warn!(
        "Webhook deactivated on PluralKit backend for user {} ({:?} != {:?}). Deactivating on our side now.",
        user,
        expected_webhook_url,
        system.webhook_url
    );

    let user_info = database::get_user_info(db_pool, user).await?;

    users::send_pluralkit_webhook_deactivated_email(db_pool, smtp_config, &user_info.email).await?;

    database::modify_user_secrets(db_pool, user, application_user_secrets, |config| {
        config.from_pluralkit_webhook_signing_token.take();
        config.enable_from_pluralkit = false;
    })
    .await?;

    log::warn!("Disabled PluralKit webhook and sent notification to user {user}");

    Ok(())
}
