use anyhow::Result;
use rocket::{State, http::Status, post};
use sqlx::PgPool;

use crate::{database, int_counter_metric, plurality, updater::UpdaterManager, users::UserId};

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
    let updater_manager: UpdaterManager = updater_manager.inner().clone();
    tokio::spawn(async move {
        match plurality::fetch_and_update_fronters(&user_id, &db_pool, &updater_manager).await {
            Ok(()) => {
                PLURALKIT_WEBHOOK_FETCH_TRIGGERED_TOTAL
                    .with_label_values(&[&user_id.to_string()])
                    .inc();
                log::info!(
                    "# | handle_pluralkit_webhook | {user_id} | fetch triggered successfully"
                );
            }
            Err(e) => {
                log::warn!("# | handle_pluralkit_webhook | {user_id} | fetch failed: {e}");
            }
        }
    });

    Ok(Status::Ok)
}

#[cfg(test)]
mod tests {
    use crate::plurality::{PluralKitWebhookEvent, PluralKitWebhookPayload};

    use super::*;
    fn create_test_payload(event: PluralKitWebhookEvent) -> PluralKitWebhookPayload {
        PluralKitWebhookPayload {
            event_type: event,
            signing_token: "test-token".to_string(),
        }
    }

    #[test]
    fn test_ping_event_serialization() {
        let payload = create_test_payload(PluralKitWebhookEvent::Ping);
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"type\":\"PING\""));
        assert!(json.contains("\"signing_token\""));
    }

    #[test]
    fn test_create_switch_event_serialization() {
        let payload = create_test_payload(PluralKitWebhookEvent::CreateSwitch);
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"type\":\"CREATE_SWITCH\""));
        assert!(json.contains("\"signing_token\""));
    }

    #[test]
    fn test_webhook_payload_deserialization() {
        let json = r#"{
            "type": "CREATE_SWITCH",
            "signing_token": "test-secret",
            "system_id": "sys123",
            "data": {
                "members": ["mem1", "mem2"],
                "timestamp": "2024-01-01T12:00:00Z"
            }
        }"#;

        let payload: PluralKitWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(
            payload.event_type,
            PluralKitWebhookEvent::CreateSwitch
        ));
        assert_eq!(payload.signing_token, "test-secret");
    }

    #[test]
    fn test_user_id_parsing() {
        // Valid UUID should parse
        let valid_uuid = "123e4567-e89b-12d3-a456-426614174000";
        assert!(UserId::try_from(valid_uuid).is_ok());

        // Invalid UUID should fail
        let invalid_uuid = "not-a-uuid";
        assert!(UserId::try_from(invalid_uuid).is_err());

        // Empty string should fail
        assert!(UserId::try_from("").is_err());
    }
}
