use crate::{
    plurality,
    users::{self},
};
use anyhow::Result;
use pluralsync_base::{
    meta,
    platforms::{DiscordActivityType, DiscordRichPresence, DiscordStatusDisplayType},
};

pub struct DiscordUpdater {
    pub last_operation_error: Option<String>,
}
impl Default for DiscordUpdater {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordUpdater {
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

    #[allow(clippy::unused_async)]
    pub async fn update_fronting_status(
        &self,
        _config: &users::UserConfigForUpdater,
        _fronts: &[plurality::Fronter],
    ) -> Result<()> {
        // fronts are sent to fronter_channel automatically by updater work loop
        Ok(())
    }
}

pub fn render_fronts_to_discord_rich_presence(
    fronters: &[plurality::Fronter],
    config: &users::UserConfigForUpdater,
) -> Result<DiscordRichPresence> {
    let short_format = plurality::FrontingFormat {
        max_length: Some(30), // seems to fit often enough without '...' truncation
        cleaning: plurality::CleanForPlatform::NoClean,
        prefix: config.status_prefix.clone(),
        status_if_no_fronters: config.status_no_fronts.clone(),
        truncate_names_to_length_if_status_too_long: config.status_truncate_names_to,
    };
    let short_fronters_string = plurality::format_fronting_status(&short_format, fronters);

    let long_format = plurality::FrontingFormat {
        max_length: Some(50), // seems to fit often enough without '...' truncation
        ..short_format
    };
    let long_fronters_string = plurality::format_fronting_status(&long_format, fronters);

    let most_recent_fronting_change: Option<i64> = fronters
        .iter()
        .filter_map(|f| f.start_time)
        .max()
        .map(|dt| dt.timestamp());

    let rich_presence = DiscordRichPresence {
        activity_type: DiscordActivityType::Playing,
        status_display_type: DiscordStatusDisplayType::Details,
        details: Some(short_fronters_string),
        details_url: Some(meta::PLURALSYNC_GITHUB_REPOSITORY_URL.to_owned()), // // future: link to fronting web url
        state: Some(long_fronters_string),
        state_url: None,
        start_time: most_recent_fronting_change,
        end_time: None,        // we can't predict when the fronting will stop
        large_image_url: None, // future: populate these fields.
        large_image_text: None,
        small_image_url: None,
        small_image_text: None,
        party_current: Some(fronters.len().try_into()?),
        party_max: None,
        button_label: None, // future: Some("View Online".to_string()), or maybe instead with a 'What's this?' buttom?
        button_url: None,   // future: link to fronting web url
    };

    log::debug!(
        "# | render_fronts_to_discord_rich_presence | {} | {:?} | {:?} | {:?}",
        config.user_id,
        &rich_presence.details,
        rich_presence.party_current,
        rich_presence.start_time
    );

    Ok(rich_presence)
}

// Formatting based on activity type: https://discord.com/developers/docs/events/gateway-events#activity-object-activity-types

// activity type: normal. display as rich presence!
// visible on yourself as well as on others. but the button isn't available for everyone to see
// OR
// let activity_type = ActivityType::Custom; // display as custom status message!
// only visible to yourself when you haven't set a custom status message manually AND when you are not hovering
// over your status on the botom left. You can also not see it on your full bio lol.
// however, it seems to be overshadowed by the normal custom status, if it's manually set by the user! to be noted!
//what about hungstatus? and is the RPC method limited or does it work scalably??? Do I need to have it verified?
// https://discord.com/developers/docs/topics/rpc
// or is this already done by this create?
// NOTE. THIS DOESN'T WORK WITH THE OFFICIAL DISCORD CLIENT! I can offer it, but let users know, that it only works with
// certain modded clients and that there is no guarantee.
