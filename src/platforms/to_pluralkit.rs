use std::collections::HashSet;

use crate::{
    int_counter_metric, metric, plurality, record_if_error, users, users::UserConfigForUpdater,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

int_counter_metric!(PLURALKIT_API_REQUESTS_TOTAL);
metric!(
    rocket_prometheus::prometheus::IntGaugeVec,
    PLURALKIT_API_RATELIMIT_REMAINING,
    "pluralkit_api_ratelimit_remaining",
    &["user_id", "scope"]
);

const TO_PLURALKIT_UPDATER_USER_AGENT: &str = concat!(
    "PluralSync/",
    env!("CARGO_PKG_VERSION"),
    " Discord: ",
    env!("USER_AGENT_DISCORD_USERNAME")
);

pub struct ToPluralKitUpdater {
    pub last_operation_error: Option<String>,
}

impl Default for ToPluralKitUpdater {
    fn default() -> Self {
        Self::new()
    }
}

impl ToPluralKitUpdater {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_operation_error: None,
        }
    }

    #[allow(clippy::unused_async)]
    pub async fn setup(&mut self, _config: &users::UserConfigForUpdater) -> Result<()> {
        // Nothing to do here for now
        Ok(())
    }

    pub async fn update_fronting_status(
        &mut self,
        config: &users::UserConfigForUpdater,
        fronts: &[plurality::Fronter],
    ) -> Result<()> {
        record_if_error!(self, update_to_pluralkit(config, fronts).await)
    }
}

async fn update_to_pluralkit(
    config: &UserConfigForUpdater,
    fronts: &[plurality::Fronter],
) -> Result<()> {
    let new_members: Vec<String> = fronts
        .iter()
        .filter_map(|f| f.pluralkit_id.clone())
        .collect();

    let response = config
        .client
        .get("https://api.pluralkit.me/v2/systems/@me/switches?limit=1")
        .header("Authorization", &config.pluralkit_token.secret)
        .header("Content-Type", "application/json")
        .header("User-Agent", TO_PLURALKIT_UPDATER_USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let existing_switches: Vec<PluralKitSwitch> =
        serde_json::from_str(&response).inspect_err(|e| {
            log::warn!(
                "# | update_to_pluralkit | {} | {} | input: {}",
                config.user_id,
                e,
                response.chars().take(500).collect::<String>()
            );
        })?;

    // if no switch exists, then no switch was ever recorded into pluralkit. then the switch-members are effectively empty.
    let existing_fronting_members: Vec<String> = existing_switches
        .into_iter()
        .map(|s| s.members)
        .collect::<Vec<Vec<String>>>()
        .first()
        .cloned()
        .unwrap_or_else(std::vec::Vec::new);

    log::debug!(
        "update_to_pluralkit | {} | existing_members={:?} | new_members={:?}",
        config.user_id,
        existing_fronting_members,
        new_members
    );

    let new_switch_members = customization_preserving_members_list_for_new_switch(
        &new_members,
        &existing_fronting_members,
    );

    if same_members(&new_switch_members, &existing_fronting_members) {
        log::debug!(
            "update_to_pluralkit | {} | No change will be propagated to PluralKit due to lists containing the same members (pk-order preservation).",
            config.user_id
        );
        return Ok(());
    }

    PLURALKIT_API_REQUESTS_TOTAL
        .with_label_values(&[&config.user_id.to_string()])
        .inc();

    let response = config
        .client
        .post("https://api.pluralkit.me/v2/systems/@me/switches")
        .header("Authorization", &config.pluralkit_token.secret)
        .header("Content-Type", "application/json")
        .header("User-Agent", TO_PLURALKIT_UPDATER_USER_AGENT)
        .json(&PluralKitSwitch {
            members: new_switch_members.clone(),
        })
        .send()
        .await?;

    measure_rate_limits(config, &response);

    response.error_for_status()?;

    log::debug!(
        "update_to_pluralkit | {} | Updated PluralKit to {:?}",
        config.user_id,
        new_switch_members
    );

    Ok(())
}

fn same_members(new_switch_members: &[String], existing_members: &[String]) -> bool {
    let new_set: HashSet<_> = new_switch_members.iter().collect();
    let existing_set: HashSet<_> = existing_members.iter().collect();
    new_set == existing_set
}

// todo. add configurations values for these things here
// For now we'll simply add the new members at the end (in the same order as from caller) and preserve the order of the old members
fn customization_preserving_members_list_for_new_switch(
    new_members: &[String],
    existing_members: &[String],
) -> Vec<String> {
    let strictly_new_members = new_members
        .iter()
        .filter(|&new_member| !existing_members.contains(new_member))
        .cloned();

    existing_members // start with existing members
        .iter()
        .filter(|&existing_member| new_members.contains(existing_member))
        .cloned() // keep only those which are also new members
        .chain(strictly_new_members) // append new members at the end
        .collect()
}

fn measure_rate_limits(config: &UserConfigForUpdater, response: &reqwest::Response) {
    let headers = response.headers();
    let rate_limit_limit = headers
        .get("X-RateLimit-Limit")
        .and_then(|v| v.to_str().ok());
    let rate_limit_remaining = headers
        .get("X-RateLimit-Remaining")
        .and_then(|v| v.to_str().ok().and_then(|s| s.parse().ok()));
    let rate_limit_reset = headers
        .get("X-RateLimit-Reset")
        .and_then(|v| v.to_str().ok());
    let rate_limit_scope = headers
        .get("X-RateLimit-Scope")
        .and_then(|v| v.to_str().ok());

    if let (Some(remaining), Some(scope)) = (rate_limit_remaining, rate_limit_scope) {
        PLURALKIT_API_RATELIMIT_REMAINING
            .with_label_values(&[&config.user_id.to_string(), scope])
            .set(remaining);
    }

    log::info!(
        "# | update_to_pluralkit | {} | updated | rate limit: limit={:?}, remaining={:?}, reset={:?}, scope={:?}",
        config.user_id,
        rate_limit_limit,
        rate_limit_remaining,
        rate_limit_reset,
        rate_limit_scope
    );
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PluralKitSwitch {
    members: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_customization_preserving_members_list_for_new_switch_simple() {
        let new_members = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let existing_members = vec!["b".to_string(), "d".to_string()];
        let result =
            customization_preserving_members_list_for_new_switch(&new_members, &existing_members);
        assert_eq!(
            result,
            vec!["b".to_string(), "a".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_customization_preserving_members_list_for_new_switch_no_new_members() {
        let new_members = vec!["a".to_string(), "b".to_string()];
        let existing_members = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result =
            customization_preserving_members_list_for_new_switch(&new_members, &existing_members);
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_customization_preserving_members_list_for_new_switch_all_new_members() {
        let new_members = vec!["a".to_string(), "b".to_string()];
        let existing_members = vec!["c".to_string(), "d".to_string()];
        let result =
            customization_preserving_members_list_for_new_switch(&new_members, &existing_members);
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_customization_preserving_members_list_for_new_switch_identical_lists() {
        let new_members = vec!["a".to_string(), "b".to_string()];
        let existing_members = vec!["a".to_string(), "b".to_string()];
        let result =
            customization_preserving_members_list_for_new_switch(&new_members, &existing_members);
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_customization_preserving_members_list_for_new_switch_order_preservation() {
        let new_members = vec!["c".to_string(), "a".to_string(), "b".to_string()];
        let existing_members = vec!["b".to_string(), "a".to_string()];
        let result =
            customization_preserving_members_list_for_new_switch(&new_members, &existing_members);
        assert_eq!(
            result,
            vec!["b".to_string(), "a".to_string(), "c".to_string()]
        );
    }
}
