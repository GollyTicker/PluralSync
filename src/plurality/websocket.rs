//! WebSocket push source handler.
//!
//! External clients push fronting status updates via this WebSocket endpoint.
//! Authentication is application-level via JWT in a `login` message.

use crate::{database, updater, users};
use anyhow::Result;
use rocket::State;
use rocket::response;
use rocket_ws;
use sqlx::PgPool;

#[get("/api/user/platform/pluralsync/events")]
pub async fn get_api_user_platform_pluralsync_events(
    _ws: rocket_ws::WebSocket,
    _shared_updaters: &State<updater::UpdaterManager>,
    _db_pool: &State<PgPool>,
    _client: &State<reqwest::Client>,
    _application_user_secrets: &State<database::ApplicationUserSecrets>,
    _jwt_secret: &State<users::ApplicationJwtSecret>,
) -> Result<(), response::Debug<anyhow::Error>> {
    // let ws = ws.config(rocket_ws::Config {
    //     write_buffer_size: 0,
    //     ..Default::default()
    // });

    todo!()
}
