use crate::database;
use crate::platforms::vrchat_auth_types;
use crate::plurality;
use crate::record_if_error;
use crate::users::UserId;
use crate::{platforms::vrchat_auth, users};
use anyhow::anyhow;
use anyhow::{Ok, Result};
use vrchatapi::{
    apis::{configuration::Configuration as VrcConfig, users_api},
    models as vrc,
};

type InitializedUpdater = (
    VrcConfig,
    vrchat_auth_types::Cookies,
    vrchat_auth_types::VRChatUserId,
    sqlx::PgPool,
    database::ApplicationUserSecrets,
);
pub struct VRChatUpdater {
    pub last_operation_error: Option<String>,
    initialized: Option<InitializedUpdater>,
}
impl Default for VRChatUpdater {
    fn default() -> Self {
        Self::new()
    }
}

impl VRChatUpdater {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_operation_error: None,
            initialized: None,
        }
    }

    pub async fn setup(
        &mut self,
        config: &users::UserConfigForUpdater,
        db_pool: &sqlx::PgPool,
        application_user_secrets: &database::ApplicationUserSecrets,
    ) -> Result<()> {
        let vrchat_init = record_if_error!(
            self,
            vrchat_auth::authenticate_vrchat_with_cookie(config).await
        );
        let (vrchat_config, cookies, vrc_user_id) = vrchat_init?;
        save_new_cookies_from_vrchat(&cookies, &config.user_id, db_pool, application_user_secrets)
            .await?; // not recording this error is OK
        self.initialized = Some((
            vrchat_config,
            cookies,
            vrc_user_id,
            db_pool.clone(),
            application_user_secrets.clone(),
        ));
        Ok(())
    }

    pub async fn update_fronting_status(
        &mut self,
        config: &users::UserConfigForUpdater,
        fronts: &[plurality::Fronter],
    ) -> Result<()> {
        let initialized_updater = record_if_error!(
            self,
            self.initialized
                .as_ref()
                .ok_or_else(|| anyhow!("update_fronting_status: Updater not initalized!"))
        );
        record_if_error!(
            self,
            update_to_vrchat(config, initialized_updater?, fronts).await
        )
    }
}

async fn update_to_vrchat(
    config: &users::UserConfigForUpdater,
    initialized_updater: &InitializedUpdater,
    fronts: &[plurality::Fronter],
) -> Result<()> {
    let fronting_format = plurality::FrontingFormat {
        max_length: Some(plurality::VRCHAT_MAX_ALLOWED_STATUS_LENGTH),
        cleaning: plurality::CleanForPlatform::VRChat,
        prefix: config.status_prefix.clone(),
        status_if_no_fronters: config.status_no_fronts.clone(),
        truncate_names_to_length_if_status_too_long: config.status_truncate_names_to,
    };

    let status_string = plurality::format_fronting_status(&fronting_format, fronts);

    set_vrchat_status(initialized_updater, &config.user_id, status_string.as_str()).await
}

async fn set_vrchat_status(
    initialized_updater: &InitializedUpdater,
    user_id: &UserId,
    status_string: &str,
) -> Result<()> {
    log::debug!("# | set_vrchat_status | {user_id}");

    let mut update_request = vrc::UpdateUserRequest::new();
    update_request.status_description = Some(status_string.to_string());

    let (vrchat_config, cookies, vrc_user_id, db_pool, application_user_secrets) =
        initialized_updater;
    users_api::update_user(
        vrchat_config,
        vrc_user_id.inner.as_str(),
        Some(update_request),
    )
    .await
    .inspect(|_| {
        log::debug!("# set_vrchat_status | {user_id} | vrchat_status_updated_to '{status_string}'");
    })?;

    save_new_cookies_from_vrchat(cookies, user_id, db_pool, application_user_secrets).await?;

    Ok(())
}

/// Save new cookies after interaction with vrchat, so that a updater-restart uses the newest cookies.
async fn save_new_cookies_from_vrchat(
    cookies: &vrchat_auth_types::Cookies,
    user_id: &UserId,
    db_pool: &sqlx::PgPool,
    application_user_secrets: &database::ApplicationUserSecrets,
) -> Result<()> {
    let serialized_cookies = vrchat_auth::serialize_cookie_store(cookies)?;
    database::modify_user_secrets(db_pool, user_id, application_user_secrets, |user_secrets| {
        user_secrets.vrchat_cookie = Some(serialized_cookies.into());
    })
    .await?;

    log::debug!("# save_new_cookies_from_vrchat | {user_id} | saved");

    Ok(())
}
