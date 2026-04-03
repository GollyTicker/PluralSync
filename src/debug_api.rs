use rocket::{State, http};
use sqlx::PgPool;

use crate::{database, meta_api::HttpResult, plurality, setup};

#[must_use]
#[allow(clippy::needless_pass_by_value)]
fn expose_internal_error(err: anyhow::Error) -> (http::Status, String) {
    (http::Status::InternalServerError, err.to_string())
}

#[post("/api/debug/cron/verify-pluralkit-webhooks")]
pub async fn post_api_debug_verify_pluralkit_webhooks(
    db_pool: &State<PgPool>,
    client: &State<reqwest::Client>,
    application_user_secrets: &State<database::ApplicationUserSecrets>,
    smtp_config: &State<setup::SmtpConfig>,
) -> HttpResult<()> {
    if !smtp_config.dangerous_local_dev_mode_print_tokens_instead_of_send_email {
        return Err((
            http::Status::Forbidden,
            "Debug endpoints are only available in development mode".to_string(),
        ));
    }

    log::info!("# | POST /api/debug/cron/verify-pluralkit-webhooks | triggered manually");

    let db_pool = db_pool.inner().clone();
    let client = client.inner().clone();
    let application_user_secrets = application_user_secrets.inner().clone();
    let smtp_config = smtp_config.inner().clone();

    plurality::verify_pluralkit_webhooks(db_pool, client, application_user_secrets, smtp_config)
        .await
        .map_err(expose_internal_error)?;

    log::info!("# | POST /api/debug/cron/verify-pluralkit-webhooks | triggered manually | ok");

    Ok(())
}
