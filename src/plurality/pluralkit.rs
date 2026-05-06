use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    database::Decrypted,
    int_counter_metric, metric,
    users::{UserConfigForUpdater, UserId},
};

use super::model::{ExcludedFronter, ExclusionReason, FilteredFronter, FilteredFronters, Fronter};
use itertools::Itertools;

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

async fn http_pluralkit_fronters(
    client: &reqwest::Client,
    pluralkit_token: &Decrypted,
    user_id: &UserId,
) -> Result<PkFronters> {
    let url = "https://api.pluralkit.me/v2/systems/@me/fronters";

    log::info!("# | fetch_current_fronters | pluralkit for {user_id}");

    let response = client
        .get(url)
        .header("Authorization", &pluralkit_token.secret)
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

pub async fn http_pluralkit_system(
    client: &reqwest::Client,
    pluralkit_token: &Decrypted,
    user_id: &UserId,
) -> Result<PkSystem> {
    let url = "https://api.pluralkit.me/v2/systems/@me";

    log::info!("# | fetch_pluralkit_system | {user_id}");

    let response = client
        .get(url)
        .header("Authorization", &pluralkit_token.secret)
        .header("User-Agent", PLURALKIT_USER_AGENT)
        .send()
        .await?
        .error_for_status()?;

    measure_rate_limits(user_id, &response);

    Ok(response.json::<PkSystem>().await?)
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
        "pluralkit rate limit: remaining={rate_limit_remaining:?}, scope={rate_limit_scope:?}"
    );

    if let (Some(remaining), Some(scope)) = (rate_limit_remaining, rate_limit_scope) {
        PLURALKIT_API_RATELIMIT_REMAINING
            .with_label_values(&[&user_id.to_string(), scope])
            .set(remaining);
    }
}

#[allow(clippy::cast_possible_wrap)]
pub async fn fetch_fronts_from_pluralkit(
    config: &UserConfigForUpdater,
) -> Result<FilteredFronters> {
    let user_id = &config.user_id;

    log::info!("# | pluralkit::fetch_fronts | {user_id}");

    PLURALKIT_API_REQUESTS_TOTAL
        .with_label_values(&[&user_id.to_string()])
        .inc();

    let pk_fronters =
        http_pluralkit_fronters(&config.client, &config.pluralkit_token, user_id).await?;

    let frontables = get_pk_members_by_privacy_rules(&pk_fronters, config);

    let (fronters, excluded): (Vec<_>, Vec<_>) =
        frontables.into_iter().partition_map(|result| match result {
            FilteredFronter::Included(f) => itertools::Either::Left(f),
            FilteredFronter::Excluded(f, reason) => {
                itertools::Either::Right(ExcludedFronter { fronter: f, reason })
            }
        });

    for f in &fronters {
        log::debug!("# | pluralkit::fetch_fronts | {user_id} | fronter[*] {f:?}");
    }

    Ok(FilteredFronters { fronters, excluded })
}

fn show_pk_member_according_to_privacy_rules(
    config: &UserConfigForUpdater,
    member: &PkMember,
    start_time: &chrono::DateTime<Utc>,
) -> FilteredFronter {
    // Check if member visibility is private - exclude entirely
    if let Some(privacy) = &member.privacy
        && privacy.visibility == PrivacyLevel::Private
    {
        let fronter = from_pk_member_to_fronter(member.clone(), config, start_time);
        return FilteredFronter::Excluded(fronter, ExclusionReason::MemberPrivacyPrivate);
    }

    let fronter = from_pk_member_to_fronter(member.clone(), config, start_time);

    if member.is_archived && !config.show_members_archived {
        return FilteredFronter::Excluded(fronter, ExclusionReason::ArchivedMemberHidden);
    }

    FilteredFronter::Included(fronter)
}

fn get_pk_members_by_privacy_rules(
    fronters: &PkFronters,
    config: &UserConfigForUpdater,
) -> Vec<FilteredFronter> {
    fronters
        .members
        .iter()
        .map(|m| show_pk_member_according_to_privacy_rules(config, m, &fronters.timestamp))
        .collect()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PkFronters {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub members: Vec<PkMember>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PkSystem {
    pub id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
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

fn redact_if_private<T>(value: T, privacy: Option<PrivacyLevel>, redacted: T) -> T {
    match privacy {
        Some(PrivacyLevel::Private) => redacted,
        Some(_) | None => value, // unspecified privacy is equivalent to public according to pluralkit
    }
}

fn from_pk_member_to_fronter(
    m: PkMember,
    config: &UserConfigForUpdater,
    current_switch_start_time: &chrono::DateTime<Utc>,
) -> Fronter {
    let name = config
        .from_pluralkit_prefer_displayname
        .then_some(m.display_name)
        .flatten()
        .unwrap_or(m.name);
    Fronter {
        fronter_id: m.id.clone(),
        name: redact_if_private(
            name,
            m.privacy.as_ref().map(|m| m.name_privacy),
            "<hidden>".to_string(),
        ),
        pronouns: redact_if_private(
            m.pronouns,
            m.privacy.as_ref().map(|m| m.pronoun_privacy),
            None,
        ),
        avatar_url: redact_if_private(
            m.avatar_url.unwrap_or_default(),
            m.privacy.as_ref().map(|m| m.avatar_privacy),
            String::new(),
        ),
        pluralkit_id: Some(m.id),
        start_time: Some(*current_switch_start_time),
        privacy_buckets: vec![],
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyLevel {
    #[default]
    Public,
    Private,
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
    use crate::users::{DiscordRichPresenceUrl, UserConfigForUpdater};
    use sqlx::types::uuid;

    fn create_test_config(prefer_displayname: bool) -> UserConfigForUpdater {
        UserConfigForUpdater {
            client: reqwest::Client::new(),
            user_id: crate::users::UserId {
                inner: uuid::Uuid::new_v4(),
            },
            simply_plural_base_url: "https://test.simplyplural.com".to_string(),
            discord_base_url: "https://discord.com".to_string(),
            status_prefix: "".to_string(),
            status_no_fronts: "".to_string(),
            status_truncate_names_to: 0,
            show_members_non_archived: true,
            show_members_archived: true,
            show_custom_fronts: true,
            respect_front_notifications_disabled: false,
            privacy_fine_grained: crate::users::PrivacyFineGrained::NoFineGrained,
            privacy_fine_grained_buckets: None,
            enable_website: false,
            enable_discord: false,
            enable_discord_status_message: false,
            enable_vrchat: false,
            enable_to_pluralkit: false,
            enable_from_pluralkit: false,
            enable_from_sp: false,
            website_url_name: "".to_string(),
            website_system_name: "".to_string(),
            simply_plural_token: Default::default(),
            discord_status_message_token: Default::default(),
            vrchat_username: Default::default(),
            vrchat_password: Default::default(),
            vrchat_cookie: Default::default(),
            pluralkit_token: Default::default(),
            from_pluralkit_webhook_signing_token: Default::default(),
            from_pluralkit_prefer_displayname: prefer_displayname,
            from_pluralkit_respect_member_visibility: false,
            from_pluralkit_respect_field_visibility: false,
            history_limit: 0,
            history_truncate_after_days: 0,
            fronter_channel_wait_increment: 0,
            discord_rich_presence_url: DiscordRichPresenceUrl::default(),
            discord_rich_presence_url_custom: None,
        }
    }

    fn create_test_member(
        id: &str,
        name: &str,
        display_name: Option<&str>,
        privacy: Option<PrivacyLevel>,
    ) -> PkMember {
        PkMember {
            id: id.to_string(),
            uuid: "test-uuid".to_string(),
            name: name.to_string(),
            display_name: display_name.map(String::from),
            color: None,
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            pronouns: Some("they/them".to_string()),
            description: None,
            birthday: None,
            tags: None,
            is_archived: false,
            privacy: privacy.map(|visibility| PkMemberFieldPrivacy {
                visibility,
                name_privacy: visibility,
                description_privacy: visibility,
                birthday_privacy: visibility,
                pronoun_privacy: visibility,
                avatar_privacy: visibility,
                banner_privacy: visibility,
                metadata_privacy: visibility,
                proxy_privacy: visibility,
            }),
        }
    }

    fn create_test_time() -> chrono::DateTime<Utc> {
        chrono::DateTime::from_timestamp(1704067200, 0).unwrap() // 2024-01-01T00:00:00Z
    }

    #[test]
    fn test_from_pk_member_to_fronter_uses_name_when_prefer_displayname_false() {
        let config = create_test_config(false);
        let member = create_test_member("mem1", "Member Name", Some("Display Name"), None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "Member Name");
        assert_eq!(fronter.fronter_id, "mem1");
    }

    #[test]
    fn test_from_pk_member_to_fronter_uses_display_name_when_available_and_preferred() {
        let config = create_test_config(true);
        let member = create_test_member("mem2", "Member Name", Some("Display Name"), None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "Display Name");
        assert_eq!(fronter.fronter_id, "mem2");
    }

    #[test]
    fn test_from_pk_member_to_fronter_falls_back_to_name_when_display_name_none() {
        let config = create_test_config(true);
        let member = create_test_member("mem3", "Member Name", None, None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "Member Name");
        assert_eq!(fronter.fronter_id, "mem3");
    }

    #[test]
    fn test_from_pk_member_to_fronter_uses_display_name_when_display_name_empty_and_preferred() {
        let config = create_test_config(true);
        let mut member = create_test_member("mem4", "Member Name", Some(""), None);
        member.display_name = Some("".to_string());
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        // Empty string is Some, so it's used
        assert_eq!(fronter.name, "");
    }

    #[test]
    fn test_from_pk_member_to_fronter_name_privacy_public() {
        let config = create_test_config(false);
        let member = create_test_member("mem5", "Member Name", None, Some(PrivacyLevel::Public));
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "Member Name");
    }

    #[test]
    fn test_from_pk_member_to_fronter_name_privacy_private() {
        let config = create_test_config(false);
        let member = create_test_member("mem6", "Member Name", None, Some(PrivacyLevel::Private));
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "<hidden>");
    }

    #[test]
    fn test_from_pk_member_to_fronter_name_privacy_private_with_display_name() {
        let config = create_test_config(true);
        let member = create_test_member(
            "mem7",
            "Member Name",
            Some("Display Name"),
            Some(PrivacyLevel::Private),
        );
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "<hidden>");
    }

    #[test]
    fn test_from_pk_member_to_fronter_pronouns_privacy_public() {
        let config = create_test_config(false);
        let member = create_test_member("mem8", "Member Name", None, Some(PrivacyLevel::Public));
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.pronouns, Some("they/them".to_string()));
    }

    #[test]
    fn test_from_pk_member_to_fronter_pronouns_privacy_private() {
        let config = create_test_config(false);
        let member = create_test_member("mem9", "Member Name", None, Some(PrivacyLevel::Private));
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.pronouns, None);
    }

    #[test]
    fn test_from_pk_member_to_fronter_avatar_privacy_public() {
        let config = create_test_config(false);
        let member = create_test_member("mem10", "Member Name", None, Some(PrivacyLevel::Public));
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.avatar_url, "https://example.com/avatar.png");
    }

    #[test]
    fn test_from_pk_member_to_fronter_avatar_privacy_private() {
        let config = create_test_config(false);
        let member = create_test_member("mem11", "Member Name", None, Some(PrivacyLevel::Private));
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.avatar_url, "");
    }

    #[test]
    fn test_from_pk_member_to_fronter_all_privacy_private() {
        let config = create_test_config(true);
        let member = create_test_member(
            "mem12",
            "Member Name",
            Some("Display Name"),
            Some(PrivacyLevel::Private),
        );
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "<hidden>");
        assert_eq!(fronter.pronouns, None);
        assert_eq!(fronter.avatar_url, "");
        assert_eq!(fronter.fronter_id, "mem12");
    }

    #[test]
    fn test_from_pk_member_to_fronter_no_privacy_specified_defaults_to_public() {
        let config = create_test_config(false);
        let member = create_test_member("mem13", "Member Name", None, None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "Member Name");
        assert_eq!(fronter.pronouns, Some("they/them".to_string()));
        assert_eq!(fronter.avatar_url, "https://example.com/avatar.png");
    }

    #[test]
    fn test_from_pk_member_to_fronter_start_time_is_set() {
        let config = create_test_config(false);
        let member = create_test_member("mem14", "Member Name", None, None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.start_time, Some(time));
    }

    #[test]
    fn test_from_pk_member_to_fronter_pluralkit_id_is_set() {
        let config = create_test_config(false);
        let member = create_test_member("mem15", "Member Name", None, None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.pluralkit_id, Some("mem15".to_string()));
    }

    #[test]
    fn test_from_pk_member_to_fronter_privacy_buckets_empty() {
        let config = create_test_config(false);
        let member = create_test_member("mem16", "Member Name", None, None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert!(fronter.privacy_buckets.is_empty());
    }

    #[test]
    fn test_from_pk_member_to_fronter_fronter_id_matches_member_id() {
        let config = create_test_config(false);
        let member = create_test_member("custom_id_123", "Member Name", None, None);
        let time = create_test_time();

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.fronter_id, "custom_id_123");
    }

    #[test]
    fn test_from_pk_member_to_fronter_mixed_field_visibility() {
        let config = create_test_config(false);
        let time = create_test_time();

        let member = PkMember {
            id: "mem_mixed".to_string(),
            uuid: "test-uuid".to_string(),
            name: "Member Name".to_string(),
            display_name: Some("Display Name".to_string()),
            color: None,
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            pronouns: Some("they/them".to_string()),
            description: None,
            birthday: None,
            tags: None,
            is_archived: false,
            privacy: Some(PkMemberFieldPrivacy {
                visibility: PrivacyLevel::Public,
                name_privacy: PrivacyLevel::Public,
                description_privacy: PrivacyLevel::Public,
                birthday_privacy: PrivacyLevel::Public,
                pronoun_privacy: PrivacyLevel::Private,
                avatar_privacy: PrivacyLevel::Public,
                banner_privacy: PrivacyLevel::Public,
                metadata_privacy: PrivacyLevel::Public,
                proxy_privacy: PrivacyLevel::Public,
            }),
        };

        let fronter = from_pk_member_to_fronter(member, &config, &time);

        assert_eq!(fronter.name, "Member Name");
        assert_eq!(fronter.pronouns, None);
        assert_eq!(fronter.avatar_url, "https://example.com/avatar.png");
        assert_eq!(fronter.fronter_id, "mem_mixed");
    }
}
