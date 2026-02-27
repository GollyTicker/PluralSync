use crate::{plurality, record_if_error, users};
use anyhow::Result;
use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Display)]
struct User {
    custom_status: Status,
}

#[derive(Serialize, Deserialize, Clone, Display)]
struct Status {
    text: String,
}

pub struct DiscordStatusMessageUpdater {
    pub last_operation_error: Option<String>,
}
impl Default for DiscordStatusMessageUpdater {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordStatusMessageUpdater {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_operation_error: None,
        }
    }

    #[allow(clippy::unused_async)]
    pub async fn setup(&self, _config: &users::UserConfigForUpdater) -> Result<()> {
        Ok(())
    }

    pub async fn update_fronting_status(
        &mut self,
        config: &users::UserConfigForUpdater,
        fronts: &[plurality::Fronter],
    ) -> Result<()> {
        record_if_error!(self, update_to_discord(config, fronts).await)
    }
}

async fn update_to_discord(
    config: &users::UserConfigForUpdater,
    fronts: &[plurality::Fronter],
) -> Result<()> {
    let fronting_format = plurality::FrontingFormat {
        max_length: Some(plurality::DISCORD_STATUS_MAX_LENGTH),
        cleaning: plurality::CleanForPlatform::NoClean,
        prefix: config.status_prefix.clone(),
        status_if_no_fronters: config.status_no_fronts.clone(),
        truncate_names_to_length_if_status_too_long: config.status_truncate_names_to,
    };

    let status_string = plurality::format_fronting_status(&fronting_format, fronts);

    set_discord_status(config, status_string).await?;

    Ok(())
}

async fn set_discord_status(
    config: &users::UserConfigForUpdater,
    status_string: String,
) -> Result<()> {
    log::debug!(
        "# | set_discord_status | {} | {status_string}",
        config.user_id
    );

    let discord_status_url = format!(
        "{}{}",
        config.discord_base_url, "/api/v10/users/@me/settings"
    );

    let body = User {
        custom_status: Status {
            text: status_string.clone(),
        },
    };

    let response = config
        .client
        .patch(discord_status_url)
        .header("Authorization", &config.discord_status_message_token.secret)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body)?)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let result_user: User = serde_json::from_str(&response).inspect_err(|e| {
        log::warn!(
            "# | set_discord_status | {} | {} | input: {}",
            config.user_id,
            e,
            response.chars().take(500).collect::<String>()
        );
    })?;

    log::debug!(
        "# | set_discord_status | {} | {status_string} | result {result_user}",
        config.user_id
    );

    Ok(())
}
