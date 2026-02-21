use anyhow::{Result, anyhow};
use sqlx::FromRow;
use std::time::Duration;

use crate::{
    config_value, config_value_if,
    database::{self, Encrypted, SecretType},
    int_counter_metric,
    users::model::UserId,
};
use serde::{Deserialize, Serialize};
use specta;

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, sqlx::Type, specta::Type,
)]
#[specta(export)]
#[sqlx(type_name = "privacy_fine_grained_enum")]
pub enum PrivacyFineGrained {
    #[default]
    NoFineGrained,
    ViaFriend,
    ViaPrivacyBuckets,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
pub struct UserConfigDbEntries<Secret, Constraints = database::InvalidConstraints>
where
    Secret: database::SecretType,
    Constraints: database::ConstraintsType,
{
    #[serde(skip)]
    pub valid_constraints: Option<Constraints>,

    // None: Use default value, if available
    // Some(x): Use this value
    pub status_prefix: Option<String>,
    pub status_no_fronts: Option<String>,
    pub status_truncate_names_to: Option<i32>,

    pub show_members_non_archived: bool,
    pub show_members_archived: bool,
    pub show_custom_fronts: bool,
    pub respect_front_notifications_disabled: bool,

    pub privacy_fine_grained: PrivacyFineGrained,
    pub privacy_fine_grained_buckets: Option<Vec<String>>,

    pub enable_website: bool,
    pub enable_discord: bool,
    pub enable_discord_status_message: bool,
    pub enable_vrchat: bool,
    pub enable_to_pluralkit: bool,

    pub website_system_name: Option<String>,
    pub website_url_name: Option<String>,

    pub history_limit: Option<i32>,
    pub history_truncate_after_days: Option<i32>,

    pub simply_plural_token: Option<Secret>,
    pub discord_status_message_token: Option<Secret>,
    pub vrchat_username: Option<Secret>,
    pub vrchat_password: Option<Secret>,
    pub vrchat_cookie: Option<Secret>,
    pub pluralkit_token: Option<Secret>,
}

impl<S: SecretType> UserConfigDbEntries<S> {
    #[must_use]
    pub fn with_defaults(&self) -> Self {
        let defaults: Self = Self::default();
        Self {
            website_system_name: self
                .website_system_name
                .clone()
                .or(defaults.website_system_name),
            website_url_name: self.website_url_name.clone().or(defaults.website_url_name),
            status_prefix: self.status_prefix.clone().or(defaults.status_prefix),
            status_no_fronts: self.status_no_fronts.clone().or(defaults.status_no_fronts),
            status_truncate_names_to: self
                .status_truncate_names_to
                .or(defaults.status_truncate_names_to),
            show_members_non_archived: self.show_members_non_archived,
            show_members_archived: self.show_members_archived,
            show_custom_fronts: self.show_custom_fronts,
            respect_front_notifications_disabled: self.respect_front_notifications_disabled,
            privacy_fine_grained: self.privacy_fine_grained,
            privacy_fine_grained_buckets: self
                .privacy_fine_grained_buckets
                .clone()
                .or(defaults.privacy_fine_grained_buckets),
            enable_website: self.enable_website,
            enable_discord: self.enable_discord,
            enable_discord_status_message: self.enable_discord_status_message,
            enable_vrchat: self.enable_vrchat,
            enable_to_pluralkit: self.enable_to_pluralkit,
            simply_plural_token: self
                .simply_plural_token
                .clone()
                .or(defaults.simply_plural_token),
            discord_status_message_token: self
                .discord_status_message_token
                .clone()
                .or(defaults.discord_status_message_token),
            vrchat_username: self.vrchat_username.clone().or(defaults.vrchat_username),
            vrchat_password: self.vrchat_password.clone().or(defaults.vrchat_password),
            vrchat_cookie: self.vrchat_cookie.clone().or(defaults.vrchat_cookie),
            pluralkit_token: self.pluralkit_token.clone().or(defaults.pluralkit_token),
            history_limit: self.history_limit.or(defaults.history_limit),
            history_truncate_after_days: self
                .history_truncate_after_days
                .or(defaults.history_truncate_after_days),
            valid_constraints: self.valid_constraints.clone(), // Constraints are not defaulted
        }
    }
}

impl<S: SecretType> Default for UserConfigDbEntries<S> {
    fn default() -> Self {
        Self {
            status_prefix: Some(String::from("F:")),
            status_no_fronts: Some(String::from("none?")),
            status_truncate_names_to: Some(3),
            show_members_non_archived: false,
            show_members_archived: false,
            show_custom_fronts: false,
            respect_front_notifications_disabled: true,
            privacy_fine_grained: PrivacyFineGrained::default(),
            privacy_fine_grained_buckets: None,
            enable_website: false,
            enable_discord: false,
            enable_discord_status_message: false,
            enable_vrchat: false,
            enable_to_pluralkit: false,
            valid_constraints: None,
            website_system_name: None,
            website_url_name: None,
            history_limit: Some(100),
            history_truncate_after_days: Some(7),
            simply_plural_token: None,
            discord_status_message_token: None,
            vrchat_username: None,
            vrchat_password: None,
            vrchat_cookie: None,
            pluralkit_token: None,
        }
    }
}

#[must_use]
pub fn metrics_config_values(user_config: &UserConfigDbEntries<Encrypted>) -> Vec<(String, bool)> {
    vec![
        ("enable_discord".to_owned(), user_config.enable_discord),
        ("enable_vrchat".to_owned(), user_config.enable_vrchat),
        ("enable_website".to_owned(), user_config.enable_website),
        (
            "enable_discord_status_message".to_owned(),
            user_config.enable_discord_status_message,
        ),
        (
            "show_members_non_archived".to_owned(),
            user_config.show_members_non_archived,
        ),
        (
            "show_members_archived".to_owned(),
            user_config.show_members_archived,
        ),
        (
            "show_custom_fronts".to_owned(),
            user_config.show_custom_fronts,
        ),
        (
            "respect_front_notifications_disabled".to_owned(),
            user_config.respect_front_notifications_disabled,
        ),
        (
            format!(
                "privacy_fine_grained_{:?}",
                user_config.privacy_fine_grained
            ),
            true,
        ),
        (
            "privacy_fine_grained_buckets_set".to_owned(),
            user_config.privacy_fine_grained_buckets.is_some(),
        ),
        (
            "status_prefix_set".to_owned(),
            user_config.status_prefix.is_some(),
        ),
        (
            "status_no_fronts_set".to_owned(),
            user_config.status_no_fronts.is_some(),
        ),
        (
            "status_truncate_names_to_set".to_owned(),
            user_config.status_truncate_names_to.is_some(),
        ),
    ]
}

/// user specific config values in the form needed for the updaters
/// !! Never convert this back into a DB entry, as it contains defaults which should not be persisted into the DB.
#[allow(clippy::struct_excessive_bools)]
pub struct UserConfigForUpdater {
    pub client: reqwest::Client,
    pub user_id: UserId,
    pub simply_plural_base_url: String,
    pub discord_base_url: String,

    // Note: v Keep this in sync with UserConfigDbEntries AND the ts-bindings! v
    pub status_prefix: String,
    pub status_no_fronts: String,
    pub status_truncate_names_to: usize,

    pub show_members_non_archived: bool,
    pub show_members_archived: bool,
    pub show_custom_fronts: bool,
    pub respect_front_notifications_disabled: bool,

    pub privacy_fine_grained: PrivacyFineGrained,
    pub privacy_fine_grained_buckets: Option<Vec<String>>,

    pub enable_website: bool,
    pub enable_discord: bool,
    pub enable_discord_status_message: bool,
    pub enable_vrchat: bool,
    pub enable_to_pluralkit: bool,

    pub website_url_name: String,
    pub website_system_name: String,

    pub simply_plural_token: database::Decrypted,
    pub discord_status_message_token: database::Decrypted,
    pub vrchat_username: database::Decrypted,
    pub vrchat_password: database::Decrypted,
    pub vrchat_cookie: database::Decrypted,
    pub pluralkit_token: database::Decrypted,
}

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct WaitSeconds {
    pub inner: Duration,
}

impl From<Duration> for WaitSeconds {
    fn from(value: Duration) -> Self {
        Self { inner: value }
    }
}

impl From<i32> for WaitSeconds {
    #[allow(clippy::cast_sign_loss)]
    fn from(secs: i32) -> Self {
        Duration::from_secs(secs as u64).into()
    }
}

int_counter_metric!(CONFIG_CREATE_WITH_STRONG_CONSTRAINTS_TOTAL_COUNT);
int_counter_metric!(CONFIG_CREATE_WITH_STRONG_CONSTRAINTS_SUCCESS_COUNT);

pub fn create_config_with_strong_constraints<Constraints>(
    user_id: &UserId,
    client: &reqwest::Client,
    db_config: &UserConfigDbEntries<database::Decrypted, Constraints>,
) -> Result<(
    UserConfigForUpdater,
    UserConfigDbEntries<database::Decrypted, database::ValidConstraints>,
)>
where
    Constraints: database::ConstraintsType,
{
    log::debug!("# | create_config_with_strong_constraints | {user_id}");
    CONFIG_CREATE_WITH_STRONG_CONSTRAINTS_TOTAL_COUNT
        .with_label_values(&[&user_id.to_string()])
        .inc();

    let db_config = database::downgrade(db_config);
    let local_config_with_defaults = db_config.with_defaults();

    let enable_discord = local_config_with_defaults.enable_discord;
    let enable_discord_status_message = local_config_with_defaults.enable_discord_status_message;
    let enable_vrchat = local_config_with_defaults.enable_vrchat;
    let enable_website = local_config_with_defaults.enable_website;
    let enable_to_pluralkit = local_config_with_defaults.enable_to_pluralkit;

    let config = UserConfigForUpdater {
        user_id: user_id.clone(),
        client: client.clone(),
        simply_plural_token: config_value!(local_config_with_defaults, simply_plural_token)?,
        simply_plural_base_url: String::from("https://api.apparyllis.com/v1"),
        status_prefix: config_value!(local_config_with_defaults, status_prefix)?,
        status_no_fronts: config_value!(local_config_with_defaults, status_no_fronts)?,
        status_truncate_names_to: config_value!(
            local_config_with_defaults,
            status_truncate_names_to
        )?
        .try_into()?,
        show_members_non_archived: local_config_with_defaults.show_members_non_archived,
        show_members_archived: local_config_with_defaults.show_members_archived,
        show_custom_fronts: local_config_with_defaults.show_custom_fronts,
        respect_front_notifications_disabled: local_config_with_defaults
            .respect_front_notifications_disabled,
        privacy_fine_grained: local_config_with_defaults.privacy_fine_grained,
        privacy_fine_grained_buckets: local_config_with_defaults
            .privacy_fine_grained_buckets
            .clone(),
        enable_website,
        enable_discord,
        enable_discord_status_message,
        enable_vrchat,
        enable_to_pluralkit,
        website_url_name: config_value_if!(
            enable_website,
            local_config_with_defaults,
            website_url_name
        )?,
        website_system_name: config_value_if!(
            enable_website,
            local_config_with_defaults,
            website_system_name
        )?,
        discord_base_url: if enable_discord_status_message {
            String::from("https://discord.com")
        } else {
            String::new()
        },
        discord_status_message_token: config_value_if!(
            enable_discord_status_message,
            local_config_with_defaults,
            discord_status_message_token
        )?,
        vrchat_username: config_value_if!(
            enable_vrchat,
            local_config_with_defaults,
            vrchat_username
        )?,
        vrchat_password: config_value_if!(
            enable_vrchat,
            local_config_with_defaults,
            vrchat_password
        )?,
        vrchat_cookie: config_value!(local_config_with_defaults, vrchat_cookie)
            .inspect(|_| log::debug!("create_config_with_strong_constraints | {user_id} | vrchat cookie found and will be used."))
            .unwrap_or_default(),
        pluralkit_token: config_value_if!(
            enable_to_pluralkit,
            local_config_with_defaults,
            pluralkit_token
        )?,
    };

    if config.privacy_fine_grained == PrivacyFineGrained::ViaPrivacyBuckets
        && config.privacy_fine_grained_buckets.is_none()
    {
        return Err(anyhow!(
            "privacy_fine_grained_buckets must be set, because privacy_fine_grained is {:?}",
            PrivacyFineGrained::ViaPrivacyBuckets
        ));
    }

    // Validate history_limit (0-1000)
    let history_limit = local_config_with_defaults
        .history_limit
        .ok_or_else(|| anyhow!("history_limit must be set (default should be 100)"))?;
    if !(0..=1000).contains(&history_limit) {
        return Err(anyhow!(
            "history_limit must be between 0 and 1000, got {history_limit}"
        ));
    }

    // Validate history_truncate_after_days (0-30)
    let history_truncate_after_days = local_config_with_defaults
        .history_truncate_after_days
        .ok_or_else(|| anyhow!("history_truncate_after_days must be set (default should be 7)"))?;
    if !(0..=30).contains(&history_truncate_after_days) {
        return Err(anyhow!(
            "history_truncate_after_days must be between 0 and 30, got {history_truncate_after_days}"
        ));
    }

    log::debug!("# | create_config_with_strong_constraints | {user_id} | created");

    let valid_config =
        database::only_use_this_function_to_mark_validation_after_you_have_actually_validated_it(
            &db_config,
        );

    log::debug!("# | create_config_with_strong_constraints | {user_id} | created | validated");
    CONFIG_CREATE_WITH_STRONG_CONSTRAINTS_SUCCESS_COUNT
        .with_label_values(&[&user_id.to_string()])
        .inc();

    Ok((config, valid_config))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use sqlx::types::uuid;

    use super::*;
    use crate::database::Decrypted;

    #[test]
    fn test_create_config_with_strong_constraints_website_disabled() {
        let user_id = UserId {
            inner: uuid::Uuid::new_v4(),
        };
        let unused_client = reqwest::Client::new();

        let mut db_config = UserConfigDbEntries::<Decrypted> {
            enable_website: false,
            website_system_name: Some("Our System".to_string()),
            website_url_name: Some("our-system".to_string()),
            status_prefix: None,
            status_no_fronts: None,
            status_truncate_names_to: None,
            show_members_non_archived: false,
            show_members_archived: false,
            show_custom_fronts: false,
            respect_front_notifications_disabled: true,
            privacy_fine_grained: PrivacyFineGrained::ViaPrivacyBuckets,
            privacy_fine_grained_buckets: Some(vec!["blabla".to_owned()]),
            enable_discord: false,
            enable_discord_status_message: false,
            enable_vrchat: false,
            enable_to_pluralkit: false,
            simply_plural_token: Some(Decrypted {
                secret: "sp_token_123".to_string(),
            }),
            discord_status_message_token: None,
            vrchat_username: None,
            vrchat_password: None,
            vrchat_cookie: None,
            valid_constraints: None,
            pluralkit_token: None,
            history_limit: Some(100),
            history_truncate_after_days: Some(7),
        };

        let (config_for_updater, _) =
            create_config_with_strong_constraints(&user_id, &unused_client, &db_config).unwrap();
        assert!(!config_for_updater.enable_website);
        assert_eq!(config_for_updater.website_system_name, "");
        assert_eq!(config_for_updater.website_url_name, "");

        // but enabling website with an empty website_system_name is disallowed work
        db_config.enable_website = true;
        db_config.website_system_name = None;

        let result = create_config_with_strong_constraints(&user_id, &unused_client, &db_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_user_config_db_entries_serialization() {
        let config = UserConfigDbEntries::<Decrypted> {
            enable_website: false,
            website_system_name: Some("Our System".to_string()),
            website_url_name: Some("our-system".to_string()),
            status_prefix: Some("SP:".to_string()),
            status_no_fronts: Some("No one fronting".to_string()),
            status_truncate_names_to: Some(5),
            show_members_non_archived: true,
            show_members_archived: false,
            show_custom_fronts: true,
            respect_front_notifications_disabled: false,
            privacy_fine_grained: PrivacyFineGrained::ViaFriend,
            privacy_fine_grained_buckets: Some(vec!["bucket1".to_string(), "bucket2".to_string()]),
            enable_discord: true,
            enable_discord_status_message: true,
            enable_vrchat: false,
            enable_to_pluralkit: true,
            simply_plural_token: Some(Decrypted {
                secret: "sp_token_123".to_string(),
            }),
            discord_status_message_token: Some(Decrypted {
                secret: "discord_status_message_token_abc".to_string(),
            }),
            vrchat_username: None,
            vrchat_password: None,
            vrchat_cookie: None,
            pluralkit_token: Some(Decrypted {
                secret: "pk_token_123".to_string(),
            }),
            history_limit: Some(100),
            history_truncate_after_days: Some(7),
            valid_constraints: None,
        };

        let json_string = serde_json::to_string_pretty(&config).unwrap();
        let expected_json = r#"{
  "status_prefix": "SP:",
  "status_no_fronts": "No one fronting",
  "status_truncate_names_to": 5,
  "show_members_non_archived": true,
  "show_members_archived": false,
  "show_custom_fronts": true,
  "respect_front_notifications_disabled": false,
  "privacy_fine_grained": "ViaFriend",
  "privacy_fine_grained_buckets": [
    "bucket1",
    "bucket2"
  ],
  "enable_website": false,
  "enable_discord": true,
  "enable_discord_status_message": true,
  "enable_vrchat": false,
  "enable_to_pluralkit": true,
  "website_system_name": "Our System",
  "website_url_name": "our-system",
  "history_limit": 100,
  "history_truncate_after_days": 7,
  "simply_plural_token": {
    "secret": "sp_token_123"
  },
  "discord_status_message_token": {
    "secret": "discord_status_message_token_abc"
  },
  "vrchat_username": null,
  "vrchat_password": null,
  "vrchat_cookie": null,
  "pluralkit_token": {
    "secret": "pk_token_123"
  }
}"#;

        assert_eq!(json_string, expected_json);
    }
}
