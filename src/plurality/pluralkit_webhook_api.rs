use anyhow::Result;
use rocket::{State, http::Status, post};
use sqlx::PgPool;

use crate::{
    database::{self, ApplicationUserSecrets},
    int_counter_metric, plurality,
    updater::UpdaterManager,
    users::UserId,
};

int_counter_metric!(PLURALKIT_WEBHOOK_REQUESTS_TOTAL);
int_counter_metric!(PLURALKIT_WEBHOOK_VALIDATION_FAILURES_TOTAL);
int_counter_metric!(PLURALKIT_WEBHOOK_RELEVANT_EVENTS_TOTAL);
int_counter_metric!(PLURALKIT_WEBHOOK_PING_EVENTS_TOTAL);
int_counter_metric!(PLURALKIT_WEBHOOK_FETCH_TRIGGERED_TOTAL);

/// `PluralKit` webhook endpoint.
///
/// This endpoint receives webhook events from `PluralKit`.
/// URL pattern: /`webhook/pluralkit/{user_id`}
///
/// The webhook signature is verified using the stored signing token.
/// On relevant events (switch changes), a full system fetch is triggered.
#[post("/api/webhook/pluralkit/<user_id>", data = "<body>")]
pub async fn post_api_webhook_pluralkit_user_id(
    user_id: &str,
    body: String,
    db_pool: &State<PgPool>,
    updater_manager: &State<UpdaterManager>,
    application_user_secrets: &State<database::ApplicationUserSecrets>,
    client: &State<reqwest::Client>,
) -> Result<Status, Status> {
    PLURALKIT_WEBHOOK_REQUESTS_TOTAL
        .with_label_values(&[user_id])
        .inc();

    let user_id = UserId::try_from(user_id).map_err(|_| Status::BadRequest)?;

    let user_config =
        database::get_user_config_with_secrets(db_pool, &user_id, client, application_user_secrets)
            .await
            .map_err(|e| {
                log::warn!("# | handle_pluralkit_webhook | {user_id} | database error: {e}");
                Status::BadRequest
            })?;

    let payload_str = &body;
    let expected_signing_token = user_config.from_pluralkit_webhook_signing_token;

    // Parse the webhook payload
    let webhook_payload: plurality::PluralKitWebhookPayload = serde_json::from_str(payload_str)
        .map_err(|e| {
            log::warn!("# | handle_pluralkit_webhook | {user_id} | parse error: {e}");
            Status::BadRequest
        })?;

    log::debug!(
        "# | handle_pluralkit_webhook | {user_id} | event: {:?}",
        webhook_payload.event_type
    );

    let provided_signing_token = webhook_payload.signing_token.clone();

    if provided_signing_token != expected_signing_token.secret {
        log::warn!("# | handle_pluralkit_webhook | token verification failed");
        return Err(Status::Unauthorized);
    }

    // Handle ping events (health check)
    if webhook_payload.event_type.is_ping() {
        PLURALKIT_WEBHOOK_PING_EVENTS_TOTAL
            .with_label_values(&[&user_id.to_string()])
            .inc();

        log::debug!("# | handle_pluralkit_webhook | {user_id} | ping ok.");

        return Ok(Status::Ok);
    }

    if webhook_payload
        .event_type
        .can_be_ignored_for_purppose_of_syncing()
    {
        log::debug!(
            "# | handle_pluralkit_webhook | {user_id} | ignoring irrelevant event: {:?}",
            webhook_payload.event_type
        );
        return Ok(Status::Ok);
    }

    PLURALKIT_WEBHOOK_RELEVANT_EVENTS_TOTAL
        .with_label_values(&[&user_id.to_string()])
        .inc();

    // asynchronously initiate the fetching and updating of fronters and members
    let user_id = user_id.clone();
    let db_pool: PgPool = db_pool.inner().clone();
    let client: reqwest::Client = client.inner().clone();
    let application_user_secrets: ApplicationUserSecrets = application_user_secrets.inner().clone();
    let updater_manager: UpdaterManager = updater_manager.inner().clone();
    tokio::spawn(async move {
        match updater_manager
            .fetch_and_update_fronters(&user_id, &client, &db_pool, &application_user_secrets)
            .await
        {
            Ok(()) => (),
            Err(e) => log::warn!("# | handle_pluralkit_webhook | {user_id} | fetch failed: {e}"),
        }
    });

    Ok(Status::Ok)
}
