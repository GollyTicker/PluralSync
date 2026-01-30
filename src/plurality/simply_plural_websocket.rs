use anyhow::{Result, anyhow};
use futures::{self, SinkExt, StreamExt};
use serde_json::{self};
use std::time::Duration;
use tokio::{net::TcpStream, select};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{self, protocol::Message},
};

use crate::{int_counter_metric, metrics::SHOULDNT_HAPPEN_BUT_IT_DID};
use std::future::Future;

int_counter_metric!(SIMPLY_PLURAL_WEBSOCKET_CONNECTION_ATTEMPTS_TOTAL);
int_counter_metric!(SIMPLY_PLURAL_WEBSOCKET_CONNECTION_ENDED_ERROR_GENERAL_TOTAL);
int_counter_metric!(SIMPLY_PLURAL_WEBSOCKET_CONNECTION_ENDED_ERROR_AUTH_TOTAL);
int_counter_metric!(SIMPLY_PLURAL_WEBSOCKET_MESSAGES_RECEIVED_TOTAL);
int_counter_metric!(SIMPLY_PLURAL_WEBSOCKET_SEMANTIC_MESSAGES_RECEIVED_TOTAL);

const WEBSOCKET_URL: &str = "wss://api.apparyllis.com/v1/socket";
const RETRY_WAIT_SECONDS: u64 = 60;
const LONGER_RETRY_AFTER_FAILED_AUTHENTICATION_SECONDS: u64 = 60 * 60;
const KEEP_ALIVE_INTERVAL: u64 = 30;

type WriteStream = futures::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type ReadStream = futures::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const SIMPLY_PLURAL_AUTH_FAILURE: &str = "Authentication against SP failed.";

/**
 * Simplifies the handling of messages from Simpl Plural.
 *
 * Takes an message handler `process_event`, and applies that handler to each event message from simply plural.
 *
 * This function takes care of the responsiblities of connection, re-connection, authentication, aborting on failures, etc.
 *
 * When the websocket reconnects, it might miss events it would have otherwise received. Hence,
 * on each successfull connect + authentication, the `on_connect` handler is called to notify such situations.
 */
#[allow(clippy::future_not_send)]
pub async fn auto_reconnecting_websocket_client_to_simply_plural<F1, F2>(
    log_prefix: &str,
    token: &str,
    process_event: impl Fn(tungstenite::Utf8Bytes) -> F1,
    on_connect: impl Fn() -> F2,
) -> !
where
    F1: Future<Output = Result<()>>,
    F2: Future<Output = Result<()>>,
{
    loop {
        log::debug!("WS {log_prefix} client starting ...");
        SIMPLY_PLURAL_WEBSOCKET_CONNECTION_ATTEMPTS_TOTAL
            .with_label_values(&[log_prefix])
            .inc();
        let wait_seconds = if let Err(e) =
            run_single_websocket_connection(log_prefix, token, &process_event, &on_connect).await
        {
            log::error!("WS {log_prefix} client error: {e}.");

            if e.to_string() == SIMPLY_PLURAL_AUTH_FAILURE {
                SIMPLY_PLURAL_WEBSOCKET_CONNECTION_ENDED_ERROR_AUTH_TOTAL
                    .with_label_values(&[log_prefix])
                    .inc();
                LONGER_RETRY_AFTER_FAILED_AUTHENTICATION_SECONDS
            } else {
                SIMPLY_PLURAL_WEBSOCKET_CONNECTION_ENDED_ERROR_GENERAL_TOTAL
                    .with_label_values(&[log_prefix])
                    .inc();
                RETRY_WAIT_SECONDS
            }
        } else {
            log::warn!("WS {log_prefix} client exited cleanly, which should not happen.",);
            SHOULDNT_HAPPEN_BUT_IT_DID
                .with_label_values(&["simply_plural_websocket_connection_ended_clean"])
                .inc();
            RETRY_WAIT_SECONDS
        };

        log::debug!("WS {log_prefix} Retrying in {wait_seconds} seconds...");
        tokio::time::sleep(Duration::from_secs(wait_seconds)).await;
    }
}

async fn run_single_websocket_connection<F1, F2>(
    log_prefix: &str,
    token: &str,
    process_event: impl Fn(tungstenite::Utf8Bytes) -> F1,
    on_connect: impl Fn() -> F2,
) -> Result<()>
where
    F1: Future<Output = Result<()>>,
    F2: Future<Output = Result<()>>,
{
    let (mut write, mut read) = create_connection(log_prefix, WEBSOCKET_URL).await?;

    authenticate(log_prefix, token, &mut write).await?;

    let mut keep_alive_interval = tokio::time::interval(Duration::from_secs(KEEP_ALIVE_INTERVAL));

    let mut authenticated = false;

    loop {
        select! {
            Some(msg) = read.next() => {
                SIMPLY_PLURAL_WEBSOCKET_MESSAGES_RECEIVED_TOTAL.with_label_values(&[log_prefix]).inc();
                match msg? {
                    Message::Text(pong) if pong == "pong" => log::debug!("WS Ok Pong."),
                    Message::Text(empty_json) if empty_json == "{}" => (),
                    Message::Text(auth_success) if auth_success.contains("Successfully authenticated") => {
                        log::info!("WS {log_prefix} Authenticated.");
                        authenticated = true;
                        on_connect().await?;
                    },
                    Message::Text(auth_failure) if auth_failure.contains("Authentication violation") => return Err(anyhow!(SIMPLY_PLURAL_AUTH_FAILURE)),
                    Message::Text(json_string) => {
                        let json_string_truncated = json_string.chars().take(100).collect::<String>();
                        log::debug!("WS {log_prefix} Received payload: '{json_string_truncated}'");
                        if !authenticated {
                            log::warn!("WS {log_prefix} Received message before authentication response: '{json_string}'");
                            SHOULDNT_HAPPEN_BUT_IT_DID
                                .with_label_values(&["simply_plural_websocket_message_before_auth"])
                                .inc();
                            return Err(anyhow!(SIMPLY_PLURAL_AUTH_FAILURE));
                        }
                        SIMPLY_PLURAL_WEBSOCKET_SEMANTIC_MESSAGES_RECEIVED_TOTAL.with_label_values(&[log_prefix]).inc();
                        process_event(json_string).await?;
                    },
                    Message::Close(_) => {
                        return Ok(());
                    }
                    unknown => {
                        log::warn!("WS {log_prefix} Unknown message '{unknown}'. Ignoring and continueing.");
                        SHOULDNT_HAPPEN_BUT_IT_DID
                            .with_label_values(&["simply_plural_websocket_unknown_message"])
                            .inc();
                    }
                }
            }
            _ = keep_alive_interval.tick() => {
                write.send(Message::Text("ping".into())).await?;
                log::debug!("WS {log_prefix} Ping sent.");
            }
        }
    }
}

async fn authenticate(log_prefix: &str, token: &str, write: &mut WriteStream) -> Result<()> {
    log::info!("WS {log_prefix} client authenticating...");
    let auth_payload = serde_json::json!({
        "op": "authenticate",
        "token": token,
    });
    write
        .send(Message::Text(auth_payload.to_string().into()))
        .await?;
    log::info!("WS {log_prefix} client authentication sent.");
    Ok(())
}

async fn create_connection(log_prefix: &str, url: &str) -> Result<(WriteStream, ReadStream)> {
    log::debug!("WS {log_prefix} client connecting to WebSocket: {url}");
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (write, read) = ws_stream.split();
    Ok((write, read))
}
