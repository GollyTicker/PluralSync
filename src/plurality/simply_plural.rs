use std::collections::HashSet;

use anyhow::{Result, anyhow};

use crate::{
    int_counter_metric, int_gauge_metric,
    plurality::{
        CustomField, CustomFront, Friend, FrontEntry, Fronter,
        GLOBAL_PLURALSYNC_ON_SIMPLY_PLURAL_USER_ID, Member,
        SIMPLY_PLURAL_VRCHAT_STATUS_NAME_FIELD_NAME,
    },
    users::{self, PrivacyFineGrained},
};

int_counter_metric!(SIMPLY_PLURAL_FETCH_FRONTS_TOTAL_COUNTER);
int_gauge_metric!(SIMPLY_PLURAL_FETCH_FRONTS_FRONTERS_COUNT);
int_gauge_metric!(SIMPLY_PLURAL_FETCH_FRONTS_ACTIVE_MEMBERS_COUNT);
int_gauge_metric!(SIMPLY_PLURAL_FETCH_FRONTS_ARCHIVED_MEMBERS_COUNT);
int_gauge_metric!(SIMPLY_PLURAL_FETCH_FRONTS_CUSTOM_FRONTS_COUNT);

#[allow(clippy::cast_possible_wrap)]
pub async fn fetch_fronts(config: &users::UserConfigForUpdater) -> Result<Vec<Fronter>> {
    let user_id = &config.user_id;

    log::info!("# | fetch_fronts | {user_id}");

    SIMPLY_PLURAL_FETCH_FRONTS_TOTAL_COUNTER
        .with_label_values(&[&user_id.to_string()])
        .inc();

    let front_entries = simply_plural_http_request_get_fronters(config).await?;

    if front_entries.is_empty() {
        SIMPLY_PLURAL_FETCH_FRONTS_FRONTERS_COUNT
            .with_label_values(&[&user_id.to_string()])
            .set(0);
        return Ok(vec![]);
    }

    let system_id = &front_entries[0].content.system_id.clone();

    let vrcsn_field_id = get_vrchat_status_name_field_id(config, system_id).await?;

    let frontables =
        get_members_and_custom_fronters_by_privacy_rules(system_id, vrcsn_field_id, config).await?;

    let fronters = filter_frontables_by_front_entries(front_entries.as_ref(), frontables.as_ref());

    for f in &fronters {
        log::debug!("# | fetch_fronts | {user_id} | fronter[*] {f:?}");
    }

    SIMPLY_PLURAL_FETCH_FRONTS_FRONTERS_COUNT
        .with_label_values(&[&user_id.to_string()])
        .set(fronters.len() as i64);

    Ok(fronters)
}

const fn show_member_according_to_privacy_rules(
    config: &users::UserConfigForUpdater,
    member_with_content: &Member,
) -> bool {
    let member: &super::MemberContent = &member_with_content.content;

    if config.respect_front_notifications_disabled && member.front_notifications_disabled {
        return false;
    }
    if member.archived {
        return config.show_members_archived;
    }

    config.show_members_non_archived
}

#[allow(clippy::cast_possible_wrap)]
async fn get_members_and_custom_fronters_by_privacy_rules(
    system_id: &str,
    vrcsn_field_id: Option<String>,
    config: &users::UserConfigForUpdater,
) -> Result<Vec<Fronter>> {
    let all_members: Vec<Member> = simply_plural_http_get_members(config, system_id).await?;

    let active_members_count = all_members.iter().filter(|m| !m.content.archived).count() as i64;

    SIMPLY_PLURAL_FETCH_FRONTS_ACTIVE_MEMBERS_COUNT
        .with_label_values(&[&config.user_id.to_string()])
        .set(active_members_count);

    SIMPLY_PLURAL_FETCH_FRONTS_ARCHIVED_MEMBERS_COUNT
        .with_label_values(&[&config.user_id.to_string()])
        .set(all_members.len() as i64 - active_members_count);

    let privacy_filtered_members = all_members
        .into_iter()
        .filter(|m| show_member_according_to_privacy_rules(config, m));

    let all_custom_fronts: Vec<CustomFront> = if config.show_custom_fronts {
        let custom_fronts = simply_plural_http_get_custom_fronts(config, system_id).await?;

        SIMPLY_PLURAL_FETCH_FRONTS_CUSTOM_FRONTS_COUNT
            .with_label_values(&[&config.user_id.to_string()])
            .set(custom_fronts.len() as i64);

        custom_fronts
    } else {
        vec![]
    };

    let all_frontables: Vec<Fronter> = privacy_filtered_members
        .into_iter()
        .map(|m| {
            let mut enriched_member = m;
            enriched_member
                .content
                .vrcsn_field_id
                .clone_from(&vrcsn_field_id);
            enriched_member
        })
        .map(Fronter::from)
        .chain(all_custom_fronts.into_iter().map(Fronter::from))
        .collect();

    let fine_grained_filtered_frontables =
        filter_frontables_by_fine_grained_privacy(system_id, config, all_frontables).await?;

    Ok(fine_grained_filtered_frontables)
}

async fn filter_frontables_by_fine_grained_privacy(
    system_id: &str,
    config: &users::UserConfigForUpdater,
    all_frontables: Vec<Fronter>,
) -> Result<Vec<Fronter>> {
    let allowed_buckets = match config.privacy_fine_grained {
        PrivacyFineGrained::NoFineGrained => return Ok(all_frontables),
        PrivacyFineGrained::ViaFriend => {
            simply_plural_http_request_get_pluralsync_assigned_buckets(config, system_id).await?
        }
        PrivacyFineGrained::ViaPrivacyBuckets => config
            .privacy_fine_grained_buckets
            .as_ref()
            .ok_or_else(|| anyhow!("privacy_fine_grained_buckets must be set"))?
            .iter()
            .cloned()
            .collect(),
    };

    let privacy_bucket_filtered = all_frontables
        .into_iter()
        .filter(|f| {
            f.privacy_buckets
                .iter()
                .any(|b| allowed_buckets.contains(b))
        })
        .collect();

    Ok(privacy_bucket_filtered)
}

fn filter_frontables_by_front_entries(
    front_entries: &[FrontEntry],
    frontables: &[Fronter],
) -> Vec<Fronter> {
    let fronters: Vec<Fronter> = frontables
        .iter()
        .filter_map(|f| {
            front_entries
                .iter()
                .find(|fe| fe.content.fronter_id == f.fronter_id)
                .map(|fe| {
                    let mut fronter_with_start_time = f.clone();
                    fronter_with_start_time.start_time = Some(fe.content.start_time);
                    fronter_with_start_time
                })
        })
        .collect();

    fronters
}

async fn simply_plural_http_request_get_fronters(
    config: &users::UserConfigForUpdater,
) -> Result<Vec<FrontEntry>> {
    log::debug!(
        "# | simply_plural_http_request_get_fronters | {}",
        config.user_id
    );

    let fronts_url = format!("{}/fronters", &config.simply_plural_base_url);
    let result = config
        .client
        .get(&fronts_url)
        .header("Authorization", &config.simply_plural_token.secret)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let result = serde_json::from_str(&result).inspect_err(|e| {
        log::warn!(
            "# | simply_plural_http_request_get_fronters | {} | {} | input: {}",
            config.user_id,
            e,
            result.chars().take(500).collect::<String>()
        );
    })?;

    Ok(result)
}

async fn get_vrchat_status_name_field_id(
    config: &users::UserConfigForUpdater,
    system_id: &String,
) -> Result<Option<String>> {
    log::debug!("# | get_vrchat_status_name_field_id | {}", config.user_id);
    let custom_fields_url = format!(
        "{}/customFields/{}",
        &config.simply_plural_base_url, system_id
    );
    let response = config
        .client
        .get(&custom_fields_url)
        .header("Authorization", &config.simply_plural_token.secret)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let custom_fields: Vec<CustomField> = serde_json::from_str(&response).inspect_err(|e| {
        log::warn!(
            "# | get_vrchat_status_name_field_id | {} | {} | input: {}",
            config.user_id,
            e,
            response.chars().take(500).collect::<String>()
        );
    })?;

    let vrchat_status_name_field = custom_fields
        .iter()
        .find(|field| field.content.name == SIMPLY_PLURAL_VRCHAT_STATUS_NAME_FIELD_NAME);

    let field_id = vrchat_status_name_field.map(|field| &field.id);

    log::debug!(
        "# | get_vrchat_status_name_field_id | {} | field_id {:?}",
        config.user_id,
        field_id
    );

    Ok(field_id.cloned())
}

async fn simply_plural_http_get_members(
    config: &users::UserConfigForUpdater,
    system_id: &str,
) -> Result<Vec<Member>> {
    log::debug!("# | simply_plural_http_get_members | {}", config.user_id);
    let fronts_url = format!("{}/members/{}", &config.simply_plural_base_url, system_id);
    let result = config
        .client
        .get(&fronts_url)
        .header("Authorization", &config.simply_plural_token.secret)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let result = serde_json::from_str(&result).inspect_err(|e| {
        log::warn!(
            "# | simply_plural_http_get_members | {e} | input: {}",
            result.chars().take(500).collect::<String>()
        );
    })?;

    Ok(result)
}

async fn simply_plural_http_get_custom_fronts(
    config: &users::UserConfigForUpdater,
    system_id: &str,
) -> Result<Vec<CustomFront>> {
    log::debug!(
        "# | simply_plural_http_get_custom_fronts | {}",
        config.user_id
    );
    let custom_fronts_url = format!(
        "{}/customFronts/{}",
        &config.simply_plural_base_url, system_id
    );
    let result = config
        .client
        .get(&custom_fronts_url)
        .header("Authorization", &config.simply_plural_token.secret)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let result = serde_json::from_str(&result).inspect_err(|e| {
        log::warn!(
            "# | simply_plural_http_get_custom_fronts | {} | {} | input: {}",
            config.user_id,
            e,
            result.chars().take(500).collect::<String>()
        );
    })?;

    Ok(result)
}

async fn simply_plural_http_request_get_pluralsync_assigned_buckets(
    config: &users::UserConfigForUpdater,
    system_id: &str,
) -> Result<HashSet<String>> {
    log::debug!(
        "# | simply_plural_http_request_get_pluralsync_assigned_buckets | {}",
        config.user_id
    );
    let friend_url = format!(
        "{}/friend/{}/{}",
        &config.simply_plural_base_url, system_id, GLOBAL_PLURALSYNC_ON_SIMPLY_PLURAL_USER_ID
    );
    let response = config
        .client
        .get(&friend_url)
        .header("Authorization", &config.simply_plural_token.secret)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let friend: Friend = serde_json::from_str(&response).inspect_err(|e| {
        log::warn!(
            "# | simply_plural_http_request_get_pluralsync_assigned_buckets | {} | {} | input: {}",
            config.user_id,
            e,
            response.chars().take(500).collect::<String>()
        );
    })?;

    let allowed_buckets = friend
        .content
        .assigned_privacy_buckets
        .into_iter()
        .collect();

    Ok(allowed_buckets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plurality::{Member, MemberContent};
    use crate::users::UserConfigForUpdater;
    use sqlx::types::uuid;

    fn create_test_config(
        respect_front_notifications_disabled: bool,
        show_members_archived: bool,
        show_members_non_archived: bool,
    ) -> UserConfigForUpdater {
        UserConfigForUpdater {
            show_members_non_archived,
            show_members_archived,
            respect_front_notifications_disabled,
            privacy_fine_grained: crate::users::PrivacyFineGrained::NoFineGrained,
            privacy_fine_grained_buckets: None,
            client: reqwest::Client::new(),
            user_id: crate::users::UserId {
                inner: uuid::Uuid::new_v4(),
            },
            simply_plural_base_url: "".to_string(),
            discord_base_url: "".to_string(),
            status_prefix: "".to_string(),
            status_no_fronts: "".to_string(),
            status_truncate_names_to: 0,
            show_custom_fronts: false,
            enable_website: false,
            enable_discord: false,
            enable_discord_status_message: false,
            enable_vrchat: false,
            enable_to_pluralkit: false,
            website_url_name: "".to_string(),
            website_system_name: "".to_string(),
            simply_plural_token: Default::default(),
            discord_status_message_token: Default::default(),
            vrchat_username: Default::default(),
            vrchat_password: Default::default(),
            vrchat_cookie: Default::default(),
            pluralkit_token: Default::default(),
            history_limit: Default::default(),
            history_truncate_after_days: Default::default(),
            fronter_channel_wait_increment: Default::default(),
        }
    }

    fn create_test_member(archived: bool, front_notifications_disabled: bool) -> Member {
        Member {
            member_id: "test_member".to_string(),
            content: MemberContent {
                name: "Test Member".to_string(),
                avatar_url: "".to_string(),
                info: serde_json::Value::Null,
                archived,
                front_notifications_disabled,
                privacy_buckets: vec![],
                vrcsn_field_id: None,
                pluralkit_id: None,
            },
        }
    }

    #[test]
    fn test_show_member_privacy_respect_front_notifications_disabled() {
        let config = create_test_config(true, true, true);
        let member = create_test_member(false, true);
        assert!(!show_member_according_to_privacy_rules(&config, &member));
    }

    #[test]
    fn test_show_member_privacy_archived_shown() {
        let config = create_test_config(false, true, true);
        let member = create_test_member(true, false);
        assert!(show_member_according_to_privacy_rules(&config, &member));
    }

    #[test]
    fn test_show_member_privacy_archived_hidden() {
        let config = create_test_config(false, false, true);
        let member = create_test_member(true, false);
        assert!(!show_member_according_to_privacy_rules(&config, &member));
    }

    #[test]
    fn test_show_member_privacy_non_archived_shown() {
        let config = create_test_config(false, true, true);
        let member = create_test_member(false, false);
        assert!(show_member_according_to_privacy_rules(&config, &member));
    }

    #[test]
    fn test_show_member_privacy_non_archived_hidden() {
        let config = create_test_config(false, true, false);
        let member = create_test_member(false, false);
        assert!(!show_member_according_to_privacy_rules(&config, &member));
    }
}
