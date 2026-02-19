use crate::metrics::SHOULDNT_HAPPEN_BUT_IT_DID;
use crate::plurality::{self};
use crate::updater::{self, change_processor};
use crate::users::UserId;
use crate::{database, users};
use crate::{int_counter_metric, metric, setup};
use anyhow::{Result, anyhow};
use pluralsync_base::updater::UpdaterStatus;
use pluralsync_base::{clock, communication};
use sqlx;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use strum::VariantNames;
use tokio::task::JoinHandle;

type SharedMutable<T> = Arc<Mutex<T>>;
type ThreadSafePerUser<T> = SharedMutable<HashMap<UserId, T>>;

type CancleableTasks = Vec<tokio::task::JoinHandle<()>>;
type FronterChannel = communication::FireAndForgetChannel<
    Vec<plurality::Fronter>,
    communication::RateLimitedMostRecentSend<Vec<plurality::Fronter>>,
>;
type ForeignStatusChannel =
    communication::FireAndForgetChannel<Option<(updater::Platform, UpdaterStatus)>>;

int_counter_metric!(UPDATER_MANAGER_RESTART_TOTAL_COUNT);
int_counter_metric!(UPDATER_MANAGER_RESTART_SUCCESS_COUNT);
metric!(
    rocket_prometheus::prometheus::IntGaugeVec,
    UPDATER_PLATFORM_STATUS,
    "updater_platform_status",
    &["user_id", "platform", "status"]
);
int_counter_metric!(UPDATER_MANAGER_SIMPLY_PLURAL_WEBSOCKET_RELEVANT_CHANGE_MESSAGE_COUNT);

#[derive(Clone)]
pub struct UpdaterManager {
    pub tasks: ThreadSafePerUser<CancleableTasks>,
    pub statuses: ThreadSafePerUser<change_processor::UserUpdatersStatuses>,
    pub fronter_channel: ThreadSafePerUser<FronterChannel>,
    pub foreign_managed_status_channel: ThreadSafePerUser<ForeignStatusChannel>,
    pub discord_status_message_available: bool,
    pub updater_start_time: ThreadSafePerUser<chrono::DateTime<chrono::Utc>>,
}

impl UpdaterManager {
    #[must_use]
    pub fn new(app_config: &setup::ApplicationConfig) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            fronter_channel: Arc::new(Mutex::new(HashMap::new())),
            discord_status_message_available: app_config.discord_status_message_updater_available,
            foreign_managed_status_channel: Arc::new(Mutex::new(HashMap::new())),
            updater_start_time: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe_fronter_channel(
        &self,
        user_id: &UserId,
    ) -> Result<communication::LatestReceiver<Vec<plurality::Fronter>>> {
        let receiver = self
            .fronter_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .get(user_id)
            .ok_or_else(|| {
                SHOULDNT_HAPPEN_BUT_IT_DID
                    .with_label_values(&["updater_manager_subcribe_fronter_channel"])
                    .inc();
                anyhow!("subscribe_fronter_channel: No fronter channel found for {user_id}")
            })?
            .subscribe();

        Ok(receiver)
    }

    pub fn fronter_channel_get_most_recent_sent_value(
        &self,
        user_id: &UserId,
    ) -> Result<Option<Vec<plurality::Fronter>>> {
        let receiver = self
            .fronter_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .get(user_id)
            .ok_or_else(|| {
                SHOULDNT_HAPPEN_BUT_IT_DID
                    .with_label_values(&["updater_manager_fronter_channel_get_most_recent_value"])
                    .inc();
                anyhow!(
                    "fronter_channel_get_most_recent_value: No fronter channel found for {user_id}"
                )
            })?
            .most_recent_sent_value();

        Ok(receiver)
    }

    #[allow(clippy::significant_drop_tightening)]
    pub fn get_foreign_status_channel(
        &self,
        user_id: &UserId,
    ) -> Result<communication::FireAndForgetChannel<Option<(updater::Platform, UpdaterStatus)>>>
    {
        let locked = self
            .foreign_managed_status_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?;

        let specific_channel = locked.get(user_id).ok_or_else(|| {
            SHOULDNT_HAPPEN_BUT_IT_DID
                .with_label_values(&["updater_manager_get_foreign_status_channel"])
                .inc();
            anyhow!("get_foreign_status_channel: No foreign status channel found for {user_id}")
        })?;

        Ok(specific_channel.clone())
    }

    pub fn get_updaters_statuses(
        &self,
        user_id: &UserId,
    ) -> Result<change_processor::UserUpdatersStatuses> {
        Ok(self
            .statuses
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .get(user_id)
            .ok_or_else(|| anyhow!("get_updaters_statuses: No updaters found!"))?
            .to_owned())
    }

    #[allow(clippy::significant_drop_tightening)]
    pub fn notify_updater_statuses(
        &self,
        user_id: &UserId,
        updater_state: change_processor::UserUpdatersStatuses,
    ) -> Result<()> {
        let mut locked = self.statuses.lock().map_err(|e| anyhow!(e.to_string()))?;

        let statuses = locked.get_mut(user_id).ok_or_else(|| {
            SHOULDNT_HAPPEN_BUT_IT_DID
                .with_label_values(&["updater_manager_notify_updater_statuses"])
                .inc();
            anyhow!("notify_updater_statuses: shouldn't happen. no statuses for user.")
        })?;

        for (p, new_status) in updater_state {
            log::debug!("# | notify_updater_statuses | {user_id} | {p} is {new_status}");

            record_status_in_metrics(user_id, p, &new_status);
            statuses.insert(p, new_status);
        }

        Ok(())
    }

    pub fn stop_updater(&self, user_id: &UserId) -> Result<()> {
        {
            let mut locked_task = self.tasks.lock().map_err(|e| anyhow!(e.to_string()))?;

            log::info!("# | stop_updater | {user_id} | aborting updaters");
            if let Some(task) = locked_task.get_mut(user_id) {
                communication::blocking_abort_and_clear_tasks(task, |x| x);
            }
            locked_task.remove(user_id);
        }

        self.fronter_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .remove(user_id);

        self.foreign_managed_status_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .remove(user_id);

        self.statuses
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .remove(user_id);

        self.updater_start_time
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .remove(user_id);

        log::info!("# | stop_updater | {user_id} | updaters stopped");

        Ok(())
    }

    #[allow(clippy::significant_drop_tightening)]
    pub fn start_updater(
        &self,
        user_id: &UserId,
        config: users::UserConfigForUpdater,
        db_pool: sqlx::PgPool,
        application_user_secrets: &database::ApplicationUserSecrets,
    ) -> Result<()> {
        let mut locked_task = self.tasks.lock().map_err(|e| anyhow!(e.to_string()))?;

        let () = self.recreate_fronter_channel(user_id)?;
        let foreign_status_updater_task = self.recreate_foreign_status_channel(user_id)?;
        let () = self.recreate_updater_statuses(user_id, &config)?;
        let simply_plural_websocket_listener_task = self
            .create_simply_plural_websocket_listener_task(
                &config,
                &db_pool,
                application_user_secrets,
            );

        let owned_self = self.to_owned();
        let application_user_secrets = application_user_secrets.clone();
        let fronter_receiver = self.subscribe_fronter_channel(user_id)?;
        let work_loop_task = tokio::spawn(async move {
            change_processor::run_listener_for_changes(
                config,
                owned_self,
                &db_pool,
                &application_user_secrets,
                fronter_receiver,
            )
            .await;
        });

        locked_task.insert(
            user_id.clone(),
            vec![
                work_loop_task,
                foreign_status_updater_task,
                simply_plural_websocket_listener_task,
            ],
        );

        self.updater_start_time
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .insert(user_id.clone(), clock::now());

        log::info!("# | start_updater | {user_id} | updaters started");
        UPDATER_MANAGER_RESTART_SUCCESS_COUNT
            .with_label_values(&[&user_id.to_string()])
            .inc();

        Ok(())
    }

    #[allow(clippy::significant_drop_tightening)]
    pub fn restart_updater(
        &self,
        user_id: &UserId,
        config: users::UserConfigForUpdater,
        db_pool: sqlx::PgPool,
        application_user_secrets: &database::ApplicationUserSecrets,
    ) -> Result<()> {
        UPDATER_MANAGER_RESTART_TOTAL_COUNT
            .with_label_values(&[&user_id.to_string()])
            .inc();

        self.stop_updater(user_id)?;
        self.start_updater(user_id, config, db_pool, application_user_secrets)?;

        log::info!("# | restart_updater | {user_id} | restarted");

        Ok(())
    }

    fn recreate_fronter_channel(&self, user_id: &UserId) -> Result<()> {
        let rate_limit_config = if cfg!(debug_assertions) {
            log::info!("recreate_fronter_channel: using debug rate limits");
            communication::RateLimitedMostRecentSend::new(
                format!("fronter_channel {user_id}"),
                chrono::Duration::milliseconds(100),
                chrono::Duration::seconds(1),
                chrono::Duration::seconds(5),
            )
        } else {
            communication::RateLimitedMostRecentSend::new(
                format!("fronter_channel {user_id}"),
                chrono::Duration::milliseconds(100),
                chrono::Duration::seconds(60),
                chrono::Duration::minutes(4),
            )
        };

        self.fronter_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .insert(
                user_id.to_owned(),
                communication::fire_and_forget_channel_with(rate_limit_config),
            ); // old value dropped
        Ok(())
    }

    // when the updaters are started, then the initial value will be fetched from simply plural into here
    fn recreate_foreign_status_channel(&self, user_id: &UserId) -> Result<JoinHandle<()>> {
        let new_channel = communication::fire_and_forget_channel();

        self.foreign_managed_status_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .insert(user_id.to_owned(), new_channel.clone()); // old value dropped

        let user_id = user_id.clone();
        let owned_self = self.clone();
        let mut receiver = new_channel.subscribe();
        let foreign_status_updater = tokio::spawn(async move {
            loop {
                if let Some(status) = receiver.recv().await {
                    match owned_self.notify_updater_statuses(&user_id, HashMap::from_iter(status)) {
                        Ok(()) => {
                            log::debug!(
                                "# | foreign_status_updater | {user_id} | status update ok."
                            );
                        }
                        Err(err) => {
                            log::warn!(
                                "# | foreign_status_updater | {user_id} | status update failed: {err}"
                            );
                        }
                    }
                } else {
                    log::debug!(
                        "# | foreign_status_updater | {user_id} | foreign_status_updater_sender_droppped terminiating_receiver"
                    );
                    break;
                }
            }
            // end of channel can sometimes happen, when asynchronously the updater is restarted
        });

        Ok(foreign_status_updater)
    }

    fn recreate_updater_statuses(
        &self,
        user_id: &UserId,
        config: &users::UserConfigForUpdater,
    ) -> Result<()> {
        let initially_disabled_status =
            updater::available_updaters(self.discord_status_message_available)
                .into_iter()
                .map(|p| (p, updater::initial_status(p, config)))
                .collect();

        self.statuses
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .insert(user_id.to_owned(), initially_disabled_status);

        Ok(())
    }

    fn create_simply_plural_websocket_listener_task(
        &self,
        config: &users::UserConfigForUpdater,
        db_pool: &sqlx::PgPool,
        application_user_secrets: &database::ApplicationUserSecrets,
    ) -> JoinHandle<()> {
        let user_id = config.user_id.clone();
        let sp_token = config.simply_plural_token.clone();
        let self2 = self.clone();
        let client = config.client.clone();
        let db_pool = db_pool.clone();
        let application_user_secrets = application_user_secrets.clone();

        tokio::spawn(async move {
            if sp_token.secret.is_empty() {
                log::info!("SP WS '{user_id}': Not creating websocket, because token is not set.");
                return;
            }
            let update_fronters_from_simply_plural = || {
                self2.fetch_and_update_fronters(
                    &user_id,
                    &client,
                    &db_pool,
                    &application_user_secrets,
                )
            };
            match update_fronters_from_simply_plural().await {
                Ok(()) => log::debug!("Initial SP update after restart OK."),
                Err(e) => log::warn!("Initial SP update after restart Err: {e}"),
            }
            plurality::auto_reconnecting_websocket_client_to_simply_plural(
                &user_id.to_string(),
                &sp_token.secret,
                async |message| {
                    let changed =
                        plurality::relevantly_changed_based_on_simply_plural_websocket_event(
                            &message,
                        )?;
                    log::debug!("SP WS payload '{user_id}': +{changed}");
                    UPDATER_MANAGER_SIMPLY_PLURAL_WEBSOCKET_RELEVANT_CHANGE_MESSAGE_COUNT
                        .with_label_values(&[&user_id.to_string()])
                        .inc();
                    if changed {
                        update_fronters_from_simply_plural().await?;
                    }
                    Ok(())
                },
                update_fronters_from_simply_plural,
            )
            .await;
        })
    }

    async fn fetch_and_update_fronters(
        &self,
        user_id: &UserId,
        client: &reqwest::Client,
        db_pool: &sqlx::Pool<sqlx::Postgres>,
        application_user_secrets: &database::ApplicationUserSecrets,
    ) -> Result<(), anyhow::Error> {
        log::debug!("# | fetch_and_update_fronters | {user_id}");

        let config = database::get_user_config_with_secrets(
            db_pool,
            user_id,
            client,
            application_user_secrets,
        )
        .await?;

        let fronters = plurality::fetch_fronts(&config).await?;
        let fronters_count = fronters.len();

        log::debug!(
            "# | fetch_and_update_fronters | {user_id} | {fronters_count} fronters fetched"
        );

        self.fronter_channel
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?
            .get_mut(user_id)
            .ok_or_else(|| {
                anyhow!("fetch_and_update_fronters: No fronter channel found for {user_id}")
            })?
            .send(fronters);

        log::debug!(
            "# | fetch_and_update_fronters | {user_id} | {fronters_count} fronters fetched | sent to channel."
        );

        Ok(())
    }

    #[allow(clippy::significant_drop_tightening)]
    fn updater_active_since(&self, user_id: &UserId) -> Result<chrono::DateTime<chrono::Utc>> {
        let locked = self
            .updater_start_time
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?;

        let start_time = locked
            .get(user_id)
            .ok_or_else(|| anyhow!("updater_active_since: No start time found for {user_id}"))?;

        Ok(*start_time)
    }
}

fn record_status_in_metrics(user_id: &UserId, p: updater::Platform, new_status: &UpdaterStatus) {
    // By using strum's EnumVariantNames, we don't need to manually maintain a list of status strings.
    for old_status in UpdaterStatus::VARIANTS {
        UPDATER_PLATFORM_STATUS
            .with_label_values(&[&user_id.to_string(), &p.to_string(), old_status])
            .set(0);
    }

    UPDATER_PLATFORM_STATUS
        .with_label_values(&[&user_id.to_string(), &p.to_string(), new_status.into()])
        .set(1);
}

/**
 * Once in a while, we restart the updaters which have been long-living.
 * Sometimes where seems to be some isolated issues where just restarting fixes it all automatically.
 *
 * We only restart the first long living updater, because we want to avoid too many updaters scheduled at the same time
 */
pub async fn restart_first_long_living_updater(
    db_pool: sqlx::PgPool,
    shared_updaters: UpdaterManager,
    application_user_secrets: database::ApplicationUserSecrets,
) -> Result<()> {
    log::debug!("restart_first_long_living_updater");

    let users = database::get_all_users(&db_pool).await?;

    for user_id in users {
        match shared_updaters.updater_active_since(&user_id) {
            Err(e) => log::warn!(
                "restart_first_long_living_updater: Could not get active_since for {user_id}: {e}"
            ),
            Ok(active_since) if is_long_lived(active_since) => {
                log::debug!(
                    "restart_first_long_living_updater | restarting {user_id} ({active_since})"
                );
                let config = database::get_user_config_with_secrets(
                    &db_pool,
                    &user_id,
                    &setup::make_client()?,
                    &application_user_secrets,
                )
                .await?;
                let _ = shared_updaters.restart_updater(
                    &user_id,
                    config,
                    db_pool,
                    &application_user_secrets,
                );
                return Ok(());
            }
            Ok(_) => (),
        }
    }

    Ok(())
}

fn is_long_lived(active_since: chrono::DateTime<chrono::Utc>) -> bool {
    let long_lived_duration = std::time::Duration::from_secs(ONE_DAY_AS_SECONDS);

    clock::now()
        .signed_duration_since(active_since)
        .to_std()
        .is_ok_and(|duration| duration > long_lived_duration)
}

const ONE_DAY_AS_SECONDS: u64 = 60 * 60 * 24;
