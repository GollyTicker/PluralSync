use std::time::Duration;

use anyhow::{Result, anyhow};
use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, ActivityType, Assets, Button, Party, StatusDisplayType, Timestamps},
};
use futures::never;
use pluralsync_base::{
    for_discord_bridge::{DiscordRichPresence, FireAndForgetChannel, ServerToBridgeSseMessage},
    updater::UpdaterStatus,
};
use serde::Deserialize;
use tokio::time::sleep;

use crate::notify_user_on_status;

// note. tell users they may need to activate rich presence sharing in their activity privacy settings. they can also customize it per server.

#[allow(clippy::unreadable_literal)]
const DISCORD_PLURALSYNC_BOT_APPLICATION_ID: u64 = 1408232222682517575;

pub async fn discord_ipc_loop(
    app: &tauri::AppHandle,
    rich_presence_channel: FireAndForgetChannel<ServerToBridgeSseMessage>,
    updater_status_channel: &mut FireAndForgetChannel<UpdaterStatus>,
) -> never::Never {
    loop {
        let err = match connect_to_discord_ipc() {
            Ok(mut client) => {
                let err = get_err(
                    activity_loop(
                        app,
                        &mut client,
                        rich_presence_channel.clone(),
                        &mut updater_status_channel.clone(),
                    )
                    .await,
                );
                log::warn!("Activity loop ended with error. Will reconnect in 5s. Error: {err}");
                err
            }
            Err(err) => {
                log::warn!("Discord IPC Connection failed. Will retry in 5s. Error: {err}");
                err
            }
        };
        updater_status_channel.send(UpdaterStatus::Error(format!(
            "Discord RPC disconnected: {err}"
        )));
        notify_user_on_status(
            app,
            format!(
                "⚠️ Discord RPC disconnected: {err} | Discord needs to be running for the bridge to work."
            ),
        );
        sleep(Duration::from_secs(5)).await;
    }
}

async fn activity_loop(
    app: &tauri::AppHandle,
    client: &mut DiscordIpcClient,
    rich_presence_channel: FireAndForgetChannel<ServerToBridgeSseMessage>,
    updater_status_channel: &mut FireAndForgetChannel<UpdaterStatus>,
) -> Result<never::Never> {
    let mut receiver = rich_presence_channel.subscribe();
    loop {
        log::info!("Waiting for PluralSync backend events...");
        if let Some(message) = receiver.recv().await {
            match message.discord_rich_presence {
                Some(drp) => set_activity(client, &drp)?,
                None => clear_activity(client)?,
            }
            updater_status_channel.send(UpdaterStatus::Running);
            notify_user_on_status(
                app,
                "Connected to PluralSync and syncing to local Discord client ✅\nIf you can't see it, then check your Discord profile and the PluralSync settings.",
            );
        } else {
            clear_activity(client)?;
            // updater status sending handled by caller
            return Err(anyhow!(
                "receiver in activity loop: sender websocket thread closed."
            ));
        }
    }
}

fn set_activity(
    client: &mut DiscordIpcClient,
    discord_presence: &DiscordRichPresence,
) -> Result<()> {
    let DiscordRichPresence {
        activity_type,
        status_display_type,
        details,
        details_url,
        state,
        state_url,
        start_time,
        end_time,
        large_image_url,
        large_image_text,
        small_image_url,
        small_image_text,
        party_current,
        party_max,
        button_label,
        button_url,
    } = discord_presence.clone();

    let mut activity = Activity::new();

    if let Some(activity_type) = activity_type_from(activity_type as u8) {
        activity = activity.activity_type(activity_type);
    }

    if let Some(status_display_type) = status_display_type_from(status_display_type as u8) {
        activity = activity.status_display_type(status_display_type);
    }

    // timestamps
    let mut timestamps = Timestamps::new();
    if let Some(start) = start_time {
        timestamps = timestamps.start(start);
    }
    if let Some(end) = end_time {
        timestamps = timestamps.end(end);
    }
    activity = activity.timestamps(timestamps);

    // state
    if let Some(state) = state.as_ref() {
        activity = activity.state(state);
    }
    if let Some(url) = state_url.as_ref() {
        activity = activity.state_url(url);
    }

    // details
    if let Some(details) = details.as_ref() {
        activity = activity.details(details);
    }
    if let Some(url) = details_url.as_ref() {
        activity = activity.details_url(url);
    }

    // assets
    let assets = {
        let mut assets = Assets::new();
        if let Some(url) = large_image_url.as_ref() {
            assets = assets.large_image(url);
        }
        if let Some(text) = large_image_text.as_ref() {
            assets = assets.large_text(text);
        }
        if let Some(url) = small_image_url.as_ref() {
            assets = assets.small_image(url);
        }
        if let Some(text) = small_image_text.as_ref() {
            assets = assets.small_text(text);
        }
        assets
    };
    activity = activity.assets(assets);

    // party
    if let (Some(party_current), Some(party_max)) = (party_current, party_max) {
        activity = activity.party(Party::new().size([party_current, party_max]));
    } else if let Some(party_current) = party_current {
        activity = activity.party(Party::new().size([party_current, 0]));
    }

    // buttom
    if let (Some(button_label), Some(button_url)) = (button_label.as_ref(), button_url.as_ref()) {
        activity = activity.buttons(vec![Button::new(button_label, button_url)]);
    }

    log::info!("Setting activity: {discord_presence:?}");

    let () = client.set_activity(activity)?;

    Ok(())
}

fn clear_activity(client: &mut DiscordIpcClient) -> Result<()> {
    log::info!("Clearing activity ...");
    let () = client.clear_activity()?;
    Ok(())
}

fn connect_to_discord_ipc() -> Result<DiscordIpcClient> {
    let mut client = DiscordIpcClient::new(DISCORD_PLURALSYNC_BOT_APPLICATION_ID.to_string());
    log::info!("Connecting to Discord IPC Client...");
    let ready: ReadyResponse = serde_json::from_value(client.connect()?)?;
    let user = ready.data.user;
    log::info!("Connected to user: {}", user.id);
    Ok(client)
}

#[derive(Clone, Deserialize, Debug)]
struct ReadyResponse {
    pub data: ReadyResponseData,
}

#[derive(Clone, Deserialize, Debug)]
struct ReadyResponseData {
    pub user: DiscordUser,
}

#[derive(Clone, Deserialize, Debug)]
struct DiscordUser {
    pub id: String,
}

const fn activity_type_from(u: u8) -> Option<ActivityType> {
    match u {
        0 => Some(ActivityType::Playing),
        2 => Some(ActivityType::Listening),
        3 => Some(ActivityType::Watching),
        4 => Some(ActivityType::Custom),
        5 => Some(ActivityType::Competing),
        _ => None,
    }
}

const fn status_display_type_from(u: u8) -> Option<StatusDisplayType> {
    match u {
        0 => Some(StatusDisplayType::Name),
        1 => Some(StatusDisplayType::State),
        2 => Some(StatusDisplayType::Details),
        _ => None,
    }
}

pub fn get_err(result: Result<never::Never>) -> anyhow::Error {
    match result {
        Err(e) => e,
        Ok(_) => unreachable!("never value observerd."),
    }
}
