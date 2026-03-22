use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sqlx;

use crate::{int_counter_metric, metric, updater, users::UserId};

int_counter_metric!(PLURALKIT_API_REQUESTS_TOTAL);
metric!(
    rocket_prometheus::prometheus::IntGaugeVec,
    PLURALKIT_API_RATELIMIT_REMAINING,
    "pluralkit_api_ratelimit_remaining",
    &["user_id", "scope"]
);

pub const PLURALKIT_USER_AGENT: &str = concat!(
    "PluralSync/",
    env!("CARGO_PKG_VERSION"),
    " Discord: ",
    env!("USER_AGENT_DISCORD_USERNAME")
);

pub async fn fetch_and_update_fronters(
    _user_id: &UserId,
    _db_pool: &sqlx::PgPool,
    _updater_manager: &updater::UpdaterManager,
) -> Result<()> {
    Err(anyhow!("not implemented"))?
}

pub async fn fetch_current_fronters(
    client: &reqwest::Client,
    pluralkit_token: &str,
    user_id: &UserId,
) -> Result<PkFronters> {
    let url = "https://api.pluralkit.me/v2/systems/@me/fronters";

    let response = client
        .get(url)
        .header("Authorization", pluralkit_token)
        .header("User-Agent", PLURALKIT_USER_AGENT)
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(PkFronters {
            members: vec![],
            timestamp: chrono::Utc::now(),
        });
    }

    let response = response.error_for_status()?;
    measure_rate_limits(user_id, &response);
    let text = response.text().await?;

    let fronters: PkFronters = serde_json::from_str(&text).inspect_err(|e| {
        log::warn!(
            "# | fetch_current_fronters | failed to parse response | {} | input: {}",
            e,
            text.chars().take(500).collect::<String>()
        );
    })?;

    Ok(fronters)
}

pub async fn fetch_system_members(
    client: &reqwest::Client,
    pluralkit_token: &str,
    user_id: &UserId,
) -> Result<Vec<PkMember>> {
    let url = "https://api.pluralkit.me/v2/systems/@me/members";

    let response = client
        .get(url)
        .header("Authorization", pluralkit_token)
        .header("User-Agent", PLURALKIT_USER_AGENT)
        .send()
        .await?
        .error_for_status()?;

    measure_rate_limits(user_id, &response);

    let text = response.text().await?;

    let members: Vec<PkMember> = serde_json::from_str(&text).inspect_err(|e| {
        log::warn!(
            "# | fetch_system_members | failed to parse response | {} | input: {}",
            e,
            text.chars().take(500).collect::<String>()
        );
    })?;

    Ok(members)
}

pub fn measure_rate_limits(user_id: &UserId, response: &reqwest::Response) {
    let headers = response.headers();
    let rate_limit_remaining = headers
        .get("X-RateLimit-Remaining")
        .and_then(|v| v.to_str().ok().and_then(|s| s.parse().ok()));
    let rate_limit_scope = headers
        .get("X-RateLimit-Scope")
        .and_then(|v| v.to_str().ok());

    log::info!(
        "pluralkit rate limit: remaining={:?}, scope={:?}",
        rate_limit_remaining,
        rate_limit_scope
    );

    if let (Some(remaining), Some(scope)) = (rate_limit_remaining, rate_limit_scope) {
        PLURALKIT_API_RATELIMIT_REMAINING
            .with_label_values(&[&user_id.to_string(), scope])
            .set(remaining);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PkFronters {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub members: Vec<PkMember>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PkMember {
    pub id: String,
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub pronouns: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub birthday: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_archived: bool,
    #[serde(default)]
    pub privacy: Option<PkMemberFieldPrivacy>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PkMemberFieldPrivacy {
    #[serde(default)]
    pub visibility: PrivacyLevel,
    #[serde(default)]
    pub name_privacy: PrivacyLevel,
    #[serde(default)]
    pub description_privacy: PrivacyLevel,
    #[serde(default)]
    pub birthday_privacy: PrivacyLevel,
    #[serde(default)]
    pub pronoun_privacy: PrivacyLevel,
    #[serde(default)]
    pub avatar_privacy: PrivacyLevel,
    #[serde(default)]
    pub banner_privacy: PrivacyLevel,
    #[serde(default)]
    pub metadata_privacy: PrivacyLevel,
    #[serde(default)]
    pub proxy_privacy: PrivacyLevel,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyLevel {
    Public,
    Private,
}

impl Default for PrivacyLevel {
    fn default() -> Self {
        Self::Public
    }
}

/// See: <https://pluralkit.me/api/dispatch>/
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluralKitWebhookEvent {
    Ping,
    UpdateSystem,
    UpdateSettings,
    CreateMember,
    UpdateMember,
    DeleteMember,
    CreateGroup,
    UpdateGroup,
    UpdateGroupMembers,
    DeleteGroup,
    LinkAccount,
    UnlinkAccount,
    UpdateSystemGuild,
    UpdateMemberGuild,
    CreateMessage,
    CreateSwitch,
    UpdateSwitch,
    DeleteSwitch,
    DeleteAllSwitches,
    SuccessfulImport,
    UpdateAutoproxy,
}

impl PluralKitWebhookEvent {
    #[must_use]
    pub const fn can_be_ignored_for_purppose_of_syncing(&self) -> bool {
        matches!(
            self,
            Self::Ping
                | Self::UpdateSettings
                | Self::CreateGroup
                | Self::UpdateGroup
                | Self::UpdateGroupMembers
                | Self::DeleteGroup
                | Self::LinkAccount
                | Self::UnlinkAccount
                | Self::UpdateSystemGuild
                | Self::UpdateMemberGuild
                | Self::UpdateAutoproxy
        )
    }

    /// Returns true if this is a ping event (for health monitoring).
    #[must_use]
    pub const fn is_ping(&self) -> bool {
        matches!(self, Self::Ping)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluralKitWebhookPayload {
    /// The event type
    #[serde(rename = "type")]
    pub event_type: PluralKitWebhookEvent,
    pub signing_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_is_ping() {
        assert!(PluralKitWebhookEvent::Ping.is_ping());
        assert!(!PluralKitWebhookEvent::CreateSwitch.is_ping());
    }

    #[test]
    fn test_payload_deserialize_switch_event() {
        let json = r#"{
            "type": "CREATE_SWITCH",
            "signing_token": "test-secret-token",
            "system_id": "sys_abc123",
            "data": {
                "members": ["mem_1", "mem_2"],
                "timestamp": "2024-01-01T00:00:00Z"
            }
        }"#;

        let payload: PluralKitWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(
            payload.event_type,
            PluralKitWebhookEvent::CreateSwitch
        ));
        assert_eq!(payload.signing_token, "test-secret-token");
    }

    #[test]
    fn test_payload_deserialize_ping() {
        let json = r#"{
            "type": "PING",
            "signing_token": "test-secret-token",
            "system_id": "sys_abc123"
        }"#;

        let payload: PluralKitWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(payload.event_type, PluralKitWebhookEvent::Ping));
        assert_eq!(payload.signing_token, "test-secret-token");
    }

    #[test]
    fn test_payload_deserialize_member_event_with_id() {
        let json = r#"{
            "type": "UPDATE_MEMBER",
            "signing_token": "test-secret-token",
            "system_id": "sys_abc123",
            "id": "mem_def456",
            "data": {
                "name": "New Member Name",
                "color": "FF5733"
            }
        }"#;

        let payload: PluralKitWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(
            payload.event_type,
            PluralKitWebhookEvent::UpdateMember
        ));
    }
}
