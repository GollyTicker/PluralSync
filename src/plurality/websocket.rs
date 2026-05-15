//! WebSocket push source handler.
//!
//! External clients push fronting status updates via this WebSocket endpoint.
//! Authentication is application-level via JWT in a `login` message.

use crate::deserialisation::{deserialize_non_empty_string, parse_rfc3339_as_option};
use crate::platforms::discord_api::is_closed;
use crate::{database, plurality, updater, users};
use anyhow::{Result, anyhow};
use pluralsync_base::clock;
use pluralsync_base::meta::PluralSyncVariantInfo;
use rocket::futures::StreamExt;
use rocket::{State, response};
use rocket_ws;
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum WsIncomingMessage {
    #[serde(rename = "login")]
    Login { user: String, auth: String },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "fronters")]
    WsFronters { data: WsFrontersData },
}

#[derive(Deserialize)]
struct WsFrontersData {
    fronters: Vec<WsFronter>,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum WsFronterPrivacy {
    Public,
    Private,
}

#[derive(Deserialize)]
struct WsFronter {
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    id: String,
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    name: String,
    #[serde(default)]
    pronouns: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default, deserialize_with = "parse_rfc3339_as_option")]
    start_time: Option<chrono::DateTime<chrono::Utc>>,
    privacy: WsFronterPrivacy,
}

#[get("/api/user/platform/pluralsync/events")]
#[allow(clippy::too_many_lines)]
pub fn get_api_user_platform_pluralsync_events(
    ws: rocket_ws::WebSocket,
    shared_updaters: &State<updater::UpdaterManager>,
    db_pool: &State<PgPool>,
    client: &State<reqwest::Client>,
    application_user_secrets: &State<database::ApplicationUserSecrets>,
    variant_info: &State<PluralSyncVariantInfo>,
    jwt_secret: &State<users::ApplicationJwtSecret>,
) -> Result<rocket_ws::Stream!['static], response::Debug<anyhow::Error>> {
    let ws = ws.config(rocket_ws::Config {
        write_buffer_size: 0,
        ..Default::default()
    });

    let variant_info_as_json_str =
        serde_json::to_string(variant_info.inner()).map_err(|e| anyhow!(e))?;

    Ok({
        let shared_updaters = shared_updaters.inner().clone();
        let db_pool = db_pool.inner().clone();
        let client = client.inner().clone();
        let application_user_secrets = application_user_secrets.inner().clone();
        let jwt_secret = jwt_secret.inner().clone();

        rocket_ws::Stream! { ws =>
            let mut ws = ws.fuse();

            // Some(...) means authenticated
            let mut user_id: Option<users::UserId> = None;

            let mut message: Option<Result<rocket_ws::Message, rocket_ws::result::Error>>;

            loop {
                message = ws.next().await;

                // we expect only text messages
                let Some(Ok(rocket_ws::Message::Text(ref message_str))) = message else {
                    log::warn!("# | websocket | received unexpected ws frame: {message:?}");
                    break;
                };

                log::debug!("# | websocket | received: {message_str}");

                let message = match serde_json::from_str::<WsIncomingMessage>(message_str) {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::warn!("# | websocket | parse error: {e}");
                        yield json_error_message("parse_error", &e.to_string());
                        if user_id.is_some() {
                            continue;
                        }
                        break;
                    }
                };

                match message {
                    WsIncomingMessage::Login { user: email, auth } => {
                        if user_id.is_some() {
                            log::warn!("# | websocket | second login attempt on authenticated connection");
                            yield json_error_message("already_authenticated", "A second login on the same connection is not allowed");
                            break;
                        }

                        let auth_token = pluralsync_base::users::JwtString { inner: auth };
                        let authenticated_user_id = match users::verify_jwt(&auth_token, &jwt_secret) {
                            Ok((_claims, user_id)) => user_id,
                            Err(e) => {
                                log::warn!("# | websocket | auth failed for user '{email}': {e}");
                                yield json_error_message("auth_failed", "Invalid or expired token");
                                break;
                            }
                        };

                        let config = match database::get_user_config_with_secrets(
                            &db_pool,
                            &authenticated_user_id,
                            &client,
                            &application_user_secrets,
                        ).await {
                            Ok(c) => c,
                            Err(e) => {
                                log::warn!("# | websocket | config fetch failed for {authenticated_user_id}: {e}");
                                yield json_error_message("server_error", &e.to_string());
                                break;
                            }
                        };

                        if !config.enable_from_websocket {
                            log::warn!(
                                "# | websocket | source mismatch for {authenticated_user_id}: enable_from_websocket=false"
                            );
                            yield json_error_message("source_mismatch", "WebSocket push source is not enabled for this user");
                            break;
                        }

                        user_id = Some(authenticated_user_id.clone());
                        log::info!(
                            "# | websocket | authenticated: {user_id:?}");

                        yield login_success_response(&variant_info_as_json_str);
                    }
                    WsIncomingMessage::Ping => {
                        if user_id.is_none() {
                            log::warn!("# | websocket | ping before login");
                            yield json_error_message("not_authenticated", "Must login first");
                            break;
                        }
                        yield pong_message();
                    }
                    WsIncomingMessage::WsFronters { data } => {
                        match user_id.as_ref() {
                            None => {
                                log::warn!("# | websocket | fronters before login");
                                yield json_error_message("not_authenticated", "Must login first");
                                break;
                            },
                            Some(user_id) => {
                                let fronters = convert_to_forwardable_fronters(data);

                                log::debug!(
                                    "# | websocket | fronters: #{} fronters for {user_id}", fronters.fronters.len()
                                );

                                match shared_updaters.notify_new_fronters(user_id, fronters) {
                                    Ok(()) => {},
                                    Err(e) =>  {
                                        log::warn!("# | websocket | failed to send fronters into channel: {e}");
                                        yield json_error_message("server_error", &e.to_string());
                                        break;
                                    }
                                }
                            }
                        }

                    }
                }
            } // loop end

            log::debug!("# | websocket | closing for {:?}", user_id);

            match message {
                None => { yield rocket_ws::Message::Close(None) },
                Some(closed) if is_closed(&closed) => (),
                _ => yield rocket_ws::Message::Close(None)
            }
        }
    })
}

fn login_success_response(variant_info_as_json_str: &str) -> rocket_ws::Message {
    let s = format!(
        "{{\"type\":\"login\",\"result\":\"success\",\"variant_info\":{variant_info_as_json_str}}}"
    );
    rocket_ws::Message::Text(s)
}

fn convert_to_forwardable_fronters(data: WsFrontersData) -> plurality::FilteredFronters {
    let now = clock::now();
    let mut fronters = Vec::new();
    let mut excluded = Vec::new();

    for fronter in data.fronters {
        let generic_fronter_privacy = fronter.privacy;
        let fronter = plurality::Fronter {
            fronter_id: fronter.id,
            name: fronter.name,
            pronouns: fronter.pronouns,
            avatar_url: fronter.avatar_url.unwrap_or_default(),
            pluralkit_id: None,
            start_time: fronter.start_time.or(Some(now)),
            privacy_buckets: Vec::new(),
        };

        match generic_fronter_privacy {
            WsFronterPrivacy::Private => {
                excluded.push(plurality::ExcludedFronter {
                    fronter,
                    reason: plurality::ExclusionReason::MemberPrivacyPrivate,
                });
            }
            WsFronterPrivacy::Public => {
                fronters.push(fronter);
            }
        }
    }

    plurality::FilteredFronters { fronters, excluded }
}

fn json_error_message(result: &str, data: &str) -> rocket_ws::Message {
    let s = format!("{{\"type\":\"error\",\"result\":\"{result}\",\"data\":\"{data}\"}}");
    rocket_ws::Message::Text(s)
}

fn pong_message() -> rocket_ws::Message {
    let json_str = "{\"type\":\"pong\"}";
    rocket_ws::Message::Text(json_str.to_owned())
}
