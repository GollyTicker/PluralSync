use crate::metrics::SHOULDNT_HAPPEN_BUT_IT_DID;
use crate::platforms::discord;
use crate::updater::Platform;
use crate::users::UserId;
use crate::{database, plurality, updater, users};
use LoopStreamControl::{Break, Continue, Yield};
use anyhow::{Result, anyhow};
use futures::never;
use pluralsync_base::communication::{self, FireAndForgetChannel, LatestReceiver};
use pluralsync_base::updater::UpdaterStatus;
use rocket::futures::StreamExt;
use rocket::{State, response};
use sqlx::PgPool;

use rocket_ws;

/// This websocket stream sends text messages of the type `ServerToBridgeSseMessage` and
/// receives messages of the type `BridgeToServerSseMessage`.
#[allow(clippy::needless_pass_by_value)]
#[get("/api/user/platform/discord/bridge-events")]
pub async fn get_api_user_platform_discord_bridge_events(
    jwt: users::Jwt,
    ws: rocket_ws::WebSocket,
    shared_updaters: &State<updater::UpdaterManager>,
    db_pool: &State<PgPool>,
    client: &State<reqwest::Client>,
    application_user_secrets: &State<database::ApplicationUserSecrets>,
) -> Result<rocket_ws::Stream!['static], response::Debug<anyhow::Error>> {
    let user_id = jwt.user_id()?;
    let user_id_c = user_id.clone();
    log::info!("# | GET /api/user/platform/discord/bridge-events | {user_id}");

    let config =
        database::get_user_config_with_secrets(db_pool, &user_id, client, application_user_secrets)
            .await?;

    let initial_fronters = shared_updaters.fronter_channel_get_most_recent_sent_value(&user_id)?;

    let fronting_channel = shared_updaters.subscribe_fronter_channel(&user_id)?;

    let foreign_status_channel = shared_updaters.get_foreign_status_channel(&user_id)?;

    let ws = ws.config(rocket_ws::Config {
        write_buffer_size: 0,
        ..Default::default()
    });

    log::info!("# | GET /api/user/platform/discord/bridge-events | {user_id} | setup");

    let stream = create_bidirection_websocket_stream_to_bridge(
        ws,
        user_id,
        config,
        initial_fronters,
        fronting_channel,
        foreign_status_channel,
    );

    log::info!(
        "# | GET /api/user/platform/discord/bridge-events | {user_id_c} | setup | returning_stream"
    );

    Ok(stream)
}

#[allow(clippy::assigning_clones)]
fn create_bidirection_websocket_stream_to_bridge(
    ws: rocket_ws::WebSocket,
    user_id: UserId,
    config: users::UserConfigForUpdater,
    initial_fronters: Option<Vec<plurality::Fronter>>,
    fronting_channel: LatestReceiver<Vec<plurality::Fronter>>,
    foreign_status_channel: FireAndForgetChannel<Option<(Platform, UpdaterStatus)>>,
) -> rocket_ws::Stream!['static] {
    let mut fronting_channel = fronting_channel;
    let mut foreign_status_channel = foreign_status_channel;
    let notify = move |s: UpdaterStatus| foreign_status_channel.send(Some((Platform::Discord, s)));

    rocket_ws::Stream! { ws =>
        let mut ws = ws.fuse();

        let ping_interval = std::time::Duration::from_secs(60);
        let mut last_received_fronters_msg = initial_fronters.clone();

        if let Some(m) = send_initial_discord_rich_presence_message(initial_fronters, &user_id, &config, notify.clone()) {
            yield m;
        }

        #[allow(clippy::needless_continue)]
        loop {
            log::debug!("# | fronters_chan <-> WS | {user_id} | Waiting...");
            let notify = notify.clone();
            tokio::select! {
                message = ws.next() => {
                    match process_message_from_bridge(message, &user_id, notify) {
                        Break => break,
                        Continue => continue,
                    }
                },
                fronters_msg = fronting_channel.recv() => {
                    last_received_fronters_msg = fronters_msg.clone();
                    match process_message_from_fronting_channel(fronters_msg, &user_id, &config, notify) {
                        Break => break,
                        Continue => continue,
                        Yield(m) => yield m,
                    }
                },
                // The websocket connection can be unstable at times and getting the TCP keepalive configured correctly wasn't easy.
                // So we just send a ping intentionally every minute and re-send the last fronters message.
                () = tokio::time::sleep(ping_interval) => {
                    log::debug!("# | fronters_chan <-> WS | {user_id} | ping re-sending last fronters.");
                    match process_message_from_fronting_channel(last_received_fronters_msg.clone(), &user_id, &config, notify) {
                        Break => break,
                        Continue => continue,
                        Yield(m) => yield m,
                    }
                }
            }
        }
        log::info!("# | fronters_chan <-> WS | {user_id} | ws_stream_end");
        yield rocket_ws::Message::Close(None);
    }
}

fn send_initial_discord_rich_presence_message(
    initial_fronters: Option<Vec<plurality::Fronter>>,
    user_id: &UserId,
    config: &users::UserConfigForUpdater,
    mut notify: impl FnMut(UpdaterStatus) -> usize,
) -> Option<rocket_ws::Message> {
    let initial_discord_rich_presence_message = initial_fronters
        .ok_or_else(|| anyhow!("No initial fronters found!"))
        .and_then(|f| discord::render_fronts_to_discord_rich_presence(f, config))
        .map(|rp| communication::ServerToBridgeSseMessage {
            discord_rich_presence: Some(rp),
        })
        .and_then(|message: communication::ServerToBridgeSseMessage| {
            serde_json::to_string(&message).map_err(|e| anyhow!(e))
        });

    match initial_discord_rich_presence_message {
        Ok(payload) => {
            log::info!("# | fronters_chan <-> WS | {user_id} | Sending initial fronters...");
            Some(rocket_ws::Message::Text(payload))
        }
        Err(e) => {
            // We want to allow the websocket connection even if the simply plural part has invalid token configured at the moment
            log::warn!(
                "# | fronters_chan <-> WS | {user_id} | No initial fronters to send. Error: {e}"
            );
            notify(UpdaterStatus::Error(
                "PluralSync-Server: No initial fronters to send.".into(),
            ));
            None
        }
    }
}

fn process_message_from_fronting_channel(
    fronters_msg: Option<Vec<plurality::Fronter>>,
    user_id: &UserId,
    config: &users::UserConfigForUpdater,
    mut notify: impl FnMut(UpdaterStatus) -> usize,
) -> LoopStreamControl<rocket_ws::Message> {
    log::info!("# | fronters_chan <-> WS | {user_id} | fronters received");
    if let Some(fronters) = fronters_msg {
        let rich_presence_result =
            discord::render_fronts_to_discord_rich_presence(fronters, config);
        match rich_presence_result {
            Ok(rich_presence) => {
                let message = communication::ServerToBridgeSseMessage {
                    discord_rich_presence: Some(rich_presence),
                };
                let payload = match rocket::serde::json::to_string(&message) {
                    Ok(p) => p,
                    Err(e) => {
                        log::warn!(
                            "# | fronters_chan <-> WS | {user_id} | fronters received | serialisation_failed {e}"
                        );
                        notify(UpdaterStatus::Error(format!(
                            "PluralSync-Server -> websocket -> PluralSync-Bridge: Server couldn't serialise fronters. Error: {e}"
                        )));
                        SHOULDNT_HAPPEN_BUT_IT_DID
                            .with_label_values(&["discord_ws_rich_presence_serialise_error"])
                            .inc();
                        return Break;
                    }
                };
                log::debug!(
                    "# | fronters_chan <-> WS | {user_id} | fronters received | sending_via_websocket"
                );
                Yield(rocket_ws::Message::Text(payload))
            }
            Err(err) => {
                log::warn!(
                    "# | fronters_chan <-> WS | {user_id} | fronters received | rendering_error {err}"
                );
                notify(UpdaterStatus::Error(format!(
                    "PluralSync-Server -> websocket -> PluralSync-Bridge: Server rendering error: {err}"
                )));
                SHOULDNT_HAPPEN_BUT_IT_DID
                    .with_label_values(&["discord_ws_rich_presence_render_error"])
                    .inc();
                Break
            }
        }
    } else {
        log::info!("# | fronters_chan <-> WS | {user_id} | fronters_chan_closed.");
        notify(UpdaterStatus::Error(
            "PluralSync-Server: Couldn't retrieve any fronters. Updater is likely being restarted."
                .into(),
        ));
        // end of channel can sometimes happen, when asynchronously the updater is restarted
        Break
    }
}

fn process_message_from_bridge(
    message: Option<std::result::Result<rocket_ws::Message, rocket_ws::result::Error>>,
    user_id: &UserId,
    mut notify: impl FnMut(UpdaterStatus) -> usize,
) -> LoopStreamControl<never::Never> {
    log::debug!("# | fronters_chan <-> WS | {user_id} | WS received {message:?}");
    match message {
        Some(close) if is_closed(&close) => {
            log::info!(
                "# | fronters_chan <-> WS | {user_id} | WS received | ws_stream_is_closed {close:?}"
            );
            notify(UpdaterStatus::Error(
                "PluralSync-Bridge -> websocket -> PluralSync-Server | No connection to bridge."
                    .to_owned(),
            ));
            Break
        }
        Some(Ok(rocket_ws::Message::Text(str))) => {
            let message: communication::BridgeToServerSseMessage = match serde_json::from_str(&str)
            {
                Ok(s) => {
                    log::debug!(
                        "# | fronters_chan <-> WS | {user_id} | WS received | deserialised to {s:?}"
                    );
                    s
                }
                Err(e) => {
                    log::warn!(
                        "# | fronters_chan <-> WS | {user_id} | WS received | deserialise_err {e}"
                    );
                    notify(UpdaterStatus::Error(format!(
                        "PluralSync-Bridge -> websocket -> PluralSync-Server | Message deserialisation error: {e}"
                    )));
                    SHOULDNT_HAPPEN_BUT_IT_DID
                        .with_label_values(&["discord_ws_deserialise_error"])
                        .inc();
                    return Break; // end on reading error
                }
            };
            notify(message.discord_updater_status);
            Continue
        }
        Some(Ok(unknown_message)) => {
            log::warn!(
                "# | fronters_chan <-> WS | {user_id} | WS received | unknown_msg_type {unknown_message:?}"
            );
            SHOULDNT_HAPPEN_BUT_IT_DID
                .with_label_values(&["discord_ws_unknown_msg"])
                .inc();
            Continue // unknown message ignored
        }
        Some(Err(_)) | None => {
            log::warn!(
                "# | fronters_chan <-> WS | {user_id} | WS received | ending_due_to_error {message:?}"
            );
            notify(UpdaterStatus::Error(format!(
                "PluralSync-Bridge -> websocket -> PluralSync-Server: Server ending due to websocket error '{message:?}'."
            )));
            Break
        } // client disconnected
    }
}

enum LoopStreamControl<Yielded> {
    Break,
    Continue,
    Yield(Yielded),
}

const fn is_closed(x: &Result<rocket_ws::Message, rocket_ws::result::Error>) -> bool {
    matches!(
        x,
        Ok(rocket_ws::Message::Close(_))
            | Err(rocket_ws::result::Error::AlreadyClosed
                | rocket_ws::result::Error::ConnectionClosed)
    )
}
