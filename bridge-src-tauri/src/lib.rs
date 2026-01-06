mod discord_bridge;
mod local_storage;
mod streaming;

use anyhow::{Result, anyhow};
use futures::stream::StreamExt;
use pluralsync_base::for_discord_bridge::{
    FireAndForgetChannel, JwtString, PluralSyncVariantInfo, ServerToBridgeSseMessage,
    UpdaterStatus, UserLoginCredentials, blocking_abort_and_clear_tasks, fire_and_forget_channel,
    license,
};
use pluralsync_base::meta;
use std::env;
use std::sync::Arc;
use tauri::{
    Emitter, Manager,
    async_runtime::{JoinHandle, Mutex},
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

const MEGABYTES: u128 = 10 ^ 6;

#[tauri::command]
async fn fetch_base_url_and_variant_info() -> Result<(String, PluralSyncVariantInfo), String> {
    let result = fetch_base_url_and_variant_info_anyhow()
        .await
        .map_err(|e| e.to_string())?;
    Ok(result)
}

async fn fetch_base_url_and_variant_info_anyhow() -> Result<(String, PluralSyncVariantInfo)> {
    let base_url = local_storage::get_base_url()?;
    let client = reqwest::Client::new();
    let variant_info_url = format!("{}{}", base_url, "/api/meta/pluralsync-variant-info");
    let variant_info = client
        .get(&variant_info_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok((base_url, variant_info))
}

fn initiate_discord_rpc_loop(
    app: tauri::AppHandle,
    rich_presence_channel: FireAndForgetChannel<ServerToBridgeSseMessage>,
    updater_status_channel: FireAndForgetChannel<UpdaterStatus>,
) {
    log::debug!("initiate_discord_rpc_loop");
    tauri::async_runtime::spawn(async move {
        let app = app;
        let mut updater_status_channel = updater_status_channel;
        discord_bridge::discord_ipc_loop(&app, rich_presence_channel, &mut updater_status_channel)
            .await;
    });
}

#[tauri::command]
async fn subscribe_to_bridge_channel(app: tauri::AppHandle, jwt: JwtString) -> Result<(), String> {
    log::debug!("subscribe_to_bridge_channel");
    subscribe_to_bridge_channel_anyhow(app, jwt)
        .await
        .map_err(|e| e.to_string())
}

async fn subscribe_to_bridge_channel_anyhow(app: tauri::AppHandle, jwt: JwtString) -> Result<()> {
    let base_url = local_storage::get_base_url()?;
    let ws_url = format!(
        "{}/api/user/platform/discord/bridge-events",
        base_url.replace("http", "ws")
    );

    let mut request = ws_url.clone().into_client_request()?;
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {}", jwt.inner).parse()?);

    log::info!("Connecting to WebSocket at {ws_url}");
    notify_user_on_status(&app, "Connecting to PluralSync to receive updates ...");

    // This websocket stream receives text messages of the type DiscordRichPresence and
    // sends messages of the type UpdaterStatus.
    let (ws_stream, _) = connect_async(request).await?;
    let (ws_send, ws_read) = ws_stream.split();

    let forwarder_task = streaming::stream_updater_status_to_ws_messages_task(app.clone(), ws_send);
    register_background_task(app.clone(), forwarder_task).await;

    let receiver_task = streaming::stream_ws_messages_to_rich_presence_task(app.clone(), ws_read);
    register_background_task(app, receiver_task).await;

    Ok(())
}

async fn register_background_task(app: tauri::AppHandle, handle: JoinHandle<()>) {
    let state = app.state::<Arc<Mutex<Vec<JoinHandle<()>>>>>();
    state.lock().await.push(handle);
}

pub fn notify_user_on_status<S: Into<String>>(app: &tauri::AppHandle, value: S) {
    let _ = app.emit("notify_user_on_status", value.into());
    // we don't care about the success.
}

pub fn restart_websocket_connection_after_retry_interval(app: &tauri::AppHandle) {
    let _ = app.emit("restart_websocket_connection_after_retry_interval", ());
    // we don't care about the success.
}

fn new_background_tasks_container() -> Arc<Mutex<Vec<JoinHandle<()>>>> {
    Arc::new(Mutex::new(vec![]))
}

async fn abort_background_task(app: tauri::AppHandle) -> () {
    log::debug!("abort_background_task");
    let state = app.state::<Arc<Mutex<Vec<JoinHandle<()>>>>>();
    let thread_shared_tasks = state.inner().clone();
    let mut locked_tasks = thread_shared_tasks.lock().await;
    blocking_abort_and_clear_tasks(
        &mut locked_tasks,
        |tauri::async_runtime::JoinHandle::Tokio(task)| task,
    );
}

#[tauri::command]
async fn login(creds: UserLoginCredentials) -> Result<JwtString, String> {
    log::debug!("login");
    login_anyhow(creds).await.map_err(|e| e.to_string())
}

async fn login_anyhow(creds: UserLoginCredentials) -> Result<JwtString> {
    let client = reqwest::Client::new();
    let base_url = local_storage::get_base_url()?;
    let login_url = format!("{}{}", base_url, "/api/user/login");

    log::info!("Attempting login: {login_url} with {:?}", &creds.email);

    let jwt_string = client
        .post(login_url)
        .json(&creds)
        .send()
        .await?
        .error_for_status()?
        .json::<JwtString>()
        .await?;

    log::info!("Login successful for {:?}", &creds.email);

    Ok(jwt_string)
}

#[tauri::command]
fn get_bridge_version() -> String {
    meta::PLURALSYNC_VERSION.to_string()
}

#[tauri::command]
async fn store_credentials(creds: UserLoginCredentials, base_url: String) -> Result<(), String> {
    log::debug!("store_credentials");
    local_storage::set_base_url(base_url).map_err(|e| e.to_string())?;
    local_storage::set_user_credentials(&creds).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn login_with_stored_credentials() -> Result<JwtString, String> {
    log::debug!("login_with_stored_credentials");
    let creds = local_storage::get_user_credentials().map_err(|e| e.to_string())?;
    let jwt_string = login(creds).await?;
    log::info!("Logged in with stored credentials.");
    Ok(jwt_string)
}
#[tauri::command]
async fn stop_and_clear_credentials(app: tauri::AppHandle) -> Result<(), String> {
    log::debug!("stop_and_clear_credentials");
    abort_background_task(app).await;
    local_storage::clear_user_credentials().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn run() -> Result<()> {
    println!("{}", license::info_text());

    let logs_dir = local_storage::get_logs_dir()?;

    let autostart_plugin = tauri_plugin_autostart::Builder::new()
        .app_name("PluralSync-Bridge")
        .build();

    let rich_presence_channel: FireAndForgetChannel<ServerToBridgeSseMessage> =
        fire_and_forget_channel();
    let updater_status_channel: FireAndForgetChannel<UpdaterStatus> = fire_and_forget_channel();

    let logging_plugin = tauri_plugin_log::Builder::default()
        .level(tauri_plugin_log::log::LevelFilter::Debug)
        .target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Webview,
        ))
        .target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Folder {
                path: logs_dir,
                file_name: None,
            },
        ))
        .max_file_size(10 * MEGABYTES)
        .build();

    let app = tauri::Builder::default()
        .plugin(autostart_plugin)
        .invoke_handler(tauri::generate_handler![
            login,
            store_credentials,
            login_with_stored_credentials,
            stop_and_clear_credentials,
            subscribe_to_bridge_channel,
            fetch_base_url_and_variant_info,
            get_bridge_version
        ])
        .manage(new_background_tasks_container())
        .manage(rich_presence_channel.clone())
        .manage(updater_status_channel.clone())
        .setup(|app| {
            app.handle().plugin(logging_plugin)?;

            move_to_system_tray(app)?;

            try_hide_window(app.app_handle());

            initiate_discord_rpc_loop(
                app.handle().clone(),
                rich_presence_channel,
                updater_status_channel,
            );
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| anyhow!(e))?;

    app.run(handle_run_event);

    Ok(())
}

fn handle_run_event(app_handle: &tauri::AppHandle, event: tauri::RunEvent) {
    match event {
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == "main" => {
            api.prevent_close();
            try_hide_window(app_handle);
        }
        _ => (),
    }
}

fn move_to_system_tray(app: &tauri::App) -> Result<(), tauri::Error> {
    // hide from taskbar
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| anyhow!("get_webview_window failed"))?;
    main_window.set_skip_taskbar(true)?;

    // build system tray
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app)?;

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "quit" => {
            app.exit(0);
        }
        "show" => {
            try_show_window(app);
        }
        _ => {}
    }
}

#[allow(clippy::needless_pass_by_value)]
fn handle_tray_icon_event(tray: &tauri::tray::TrayIcon, event: tauri::tray::TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        try_show_window(tray.app_handle());
    }
}

// ignores failures
fn try_hide_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window
            .minimize()
            .inspect_err(|e| log::warn!("window.minimize failed: {e}"));
        let _ = window
            .hide()
            .inspect_err(|e| log::warn!("window.hide failed: {e}"));
    } else {
        log::warn!("try_hide_window: main window not found!");
    }
}

// ignores failures
fn try_show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window
            .unminimize()
            .inspect_err(|e| log::warn!("window.unminimize failed: {e}"));
        let _ = window
            .show()
            .inspect_err(|e| log::warn!("window.show failed: {e}"));
        let _ = window
            .set_focus()
            .inspect_err(|e| log::warn!("window.set_focus failed: {e}"));
    } else {
        log::warn!("try_show_window: main window not found!");
    }
}
