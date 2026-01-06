use crate::database;
use crate::meta_api::{HttpResult, expose_internal_error};
use crate::setup;
use crate::updater::{change_processor, manager};
use crate::users;
use crate::users::UserId;
use anyhow::Result;
use rocket::{State, serde::json::Json};
use sqlx::PgPool;

#[allow(clippy::needless_pass_by_value)]
#[get("/api/updaters/status")]
pub fn get_api_updaters_status(
    shared_updaters: &State<manager::UpdaterManager>,
    jwt: users::Jwt,
) -> HttpResult<Json<change_processor::UserUpdatersStatuses>> {
    let user_id = jwt.user_id().map_err(expose_internal_error)?;

    log::info!("# | GET /api/updaters/status | {user_id}");

    let updaters_state: change_processor::UserUpdatersStatuses = shared_updaters
        .get_updaters_statuses(&user_id)
        .map_err(expose_internal_error)?;

    log::info!("# | GET /api/updaters/status | {user_id} | retrieved");

    Ok(Json(updaters_state))
}

pub async fn restart_all_user_updaters_for_app_startups(
    setup: setup::ApplicationSetup,
) -> Result<()> {
    log::info!("# | restart_all_user_updaters_for_app_startups");

    let all_users = database::get_all_users(&setup.db_pool).await?;

    log::info!("# | restart_all_user_updaters_for_app_startups | all_users {all_users:?}");

    for user_id in all_users {
        let _: Result<()> = restart_updater_for_user(
            &user_id,
            &setup.db_pool,
            &setup.application_user_secrets,
            &setup.client,
            &setup.shared_updaters,
        )
        .await
        .or_else(|e| {
            log::warn!(
                "# restart_all_user_updaters_for_app_startups | {user_id} failed. skipping. {e}"
            );
            Ok(())
        });
        // allow failure by not checking the result.
    }

    log::info!("# | restart_all_user_updaters_for_app_startups | all_users | ok");

    Ok(())
}

pub async fn restart_updater_for_user(
    user_id: &UserId,
    db_pool: &PgPool,
    application_user_secrets: &database::ApplicationUserSecrets,
    client: &reqwest::Client,
    shared_updaters: &manager::UpdaterManager,
) -> Result<()> {
    log::info!("# | restart_updater_for_user | {user_id}");

    let config =
        database::get_user_config_with_secrets(db_pool, user_id, client, application_user_secrets)
            .await?;

    let () = shared_updaters.restart_updater(
        user_id,
        config,
        db_pool.clone(),
        application_user_secrets,
    )?;

    log::info!("# | restart_updater_for_user | {user_id} | ok");

    Ok(())
}
