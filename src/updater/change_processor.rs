use pluralsync_base::clock;
use pluralsync_base::communication::LatestReceiver;
use pluralsync_base::updater::UpdaterStatus;
use std::collections::HashMap;

use crate::plurality::fronting_status::{CleanForPlatform, FrontingFormat, format_fronting_status};
use crate::updater::platforms::{Platform, Updater};
use crate::updater::{manager, platforms};
use crate::users::UserId;
use crate::{database, int_counter_metric, plurality, users};
use anyhow::Result;

// NOTE: specta::Type is manually exported in bindings
pub type UserUpdatersStatuses = HashMap<Platform, UpdaterStatus>;
type UserUpdaters = HashMap<Platform, Updater>;

int_counter_metric!(UPDATER_PROCESS_START_TOTAL);
int_counter_metric!(UPDATER_PROCESS_SUCCESS_TOTAL);
int_counter_metric!(UPDATER_PROCESS_UNEXPECTED_STOP_TOTAL);

pub async fn run_listener_for_changes(
    config: users::UserConfigForUpdater,
    shared_updaters: manager::UpdaterManager,
    db_pool: &sqlx::PgPool,
    application_user_secrets: &database::ApplicationUserSecrets,
    fronter_receiver: LatestReceiver<Vec<plurality::Fronter>>,
) -> () {
    let user_id = &config.user_id;
    log::debug!("# | updater run_loop | {user_id}");

    let mut fronter_receiver = fronter_receiver;

    let mut updaters: UserUpdaters =
        platforms::pluralsync_server_updaters(shared_updaters.discord_status_message_available)
            .iter()
            .map(|platform| (platform.to_owned(), Updater::new(platform)))
            .collect();

    for u in updaters.values_mut() {
        if u.enabled(&config) {
            log_error_and_continue(
                &u.platform().to_string(),
                u.setup(&config, db_pool, application_user_secrets).await,
                &config,
            );
        }
    }

    log_error_and_continue(
        "update statues",
        shared_updaters.notify_updater_statuses(user_id, get_statuses(&updaters, &config)),
        &config,
    );

    while let Some(fronters) = fronter_receiver.recv().await {
        log::debug!(
            "# | updater processing change | {} | ======================= UTC {}",
            config.user_id,
            clock::now().format("%Y-%m-%d %H:%M:%S")
        );
        UPDATER_PROCESS_START_TOTAL
            .with_label_values(&[&user_id.to_string()])
            .inc();

        log_error_and_continue(
            "Updater Logic",
            loop_logic(&config, &mut updaters, &fronters, db_pool).await,
            &config,
        );

        log_error_and_continue(
            "update statues",
            shared_updaters.notify_updater_statuses(user_id, get_statuses(&updaters, &config)),
            &config,
        );

        log::debug!(
            "# | updater processing change | {user_id} | Waiting for next update trigger...",
        );
        UPDATER_PROCESS_SUCCESS_TOTAL
            .with_label_values(&[&user_id.to_string()])
            .inc();
    }

    log::debug!("# | updater | {user_id} | end of fronter channel");
    // this only happens, when the updater is being restarted and the channel was asynchronously closed.
}

fn get_statuses(
    updaters: &UserUpdaters,
    config: &users::UserConfigForUpdater,
) -> UserUpdatersStatuses {
    updaters
        .iter()
        .map(|(k, u)| (k.to_owned(), u.status(config)))
        .collect()
}

async fn loop_logic(
    config: &users::UserConfigForUpdater,
    updaters: &mut UserUpdaters,
    fronters: &[plurality::Fronter],
    db_pool: &sqlx::PgPool,
) -> Result<()> {
    for updater in updaters.values_mut() {
        if updater.enabled(config) {
            log_error_and_continue(
                &updater.platform().to_string(),
                updater.update_fronting_status(config, fronters).await,
                config,
            );
        }
    }

    append_new_fronters_to_history(config, fronters, db_pool).await;

    Ok(())
}

async fn append_new_fronters_to_history(
    config: &users::UserConfigForUpdater,
    fronters: &[plurality::Fronter],
    db_pool: &sqlx::PgPool,
) {
    let fronting_format = FrontingFormat {
        max_length: None,
        cleaning: CleanForPlatform::NoClean,
        prefix: config.status_prefix.clone(),
        status_if_no_fronters: config.status_no_fronts.clone(),
        truncate_names_to_length_if_status_too_long: config.status_truncate_names_to,
    };
    let status_text = format_fronting_status(&fronting_format, fronters);

    log_error_and_continue(
        "store history",
        store_history_entry(
            db_pool,
            &config.user_id,
            &status_text,
            config.history_limit,
            config.history_truncate_after_days,
        )
        .await,
        config,
    );
}

async fn store_history_entry(
    pool: &sqlx::PgPool,
    user_id: &UserId,
    status_text: &str,
    history_limit: usize,
    history_truncate_after_days: usize,
) -> Result<(), anyhow::Error> {
    database::insert_history_entry(pool, user_id, status_text).await?;
    database::prune_history(pool, user_id, history_limit, history_truncate_after_days).await?;
    Ok(())
}

fn log_error_and_continue(
    loop_part_name: &str,
    res: Result<()>,
    config: &users::UserConfigForUpdater,
) {
    match res {
        Ok(()) => log::info!(
            "# | updater run_loop | {} | {loop_part_name} | ok",
            config.user_id
        ),
        Err(err) => log::warn!(
            "# | updater run_loop | {} | {loop_part_name} | skipping due to error {err}",
            config.user_id
        ),
    }
}
