use crate::database;
use crate::meta_api::HttpResult;
use crate::meta_api::expose_internal_error;
use crate::setup::SmtpConfig;
use crate::updater;
use crate::users::SecretHashOptions;
use crate::users::auth;
use crate::users::email;
use crate::users::jwt;
use crate::users::model::UserId;
use pluralsync_base::users::Email;
use pluralsync_base::users::EmailVerificationToken;
use pluralsync_base::users::UserProvidedPassword;
use rocket::http::Status;
use rocket::{State, delete, get, post, serde::json::Json};
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Deserialize, specta::Type)]
pub struct ChangeEmailRequest {
    pub new_email: Email,
}

#[post("/api/user/email/change", data = "<request>")]
pub async fn post_api_user_email_change(
    db_pool: &State<PgPool>,
    smtp_config: &State<SmtpConfig>,
    app_user_secrets: &State<database::ApplicationUserSecrets>,
    jwt: jwt::Jwt,
    request: Json<ChangeEmailRequest>,
) -> HttpResult<()> {
    let user_id = jwt.user_id().map_err(expose_internal_error)?;
    log::info!("# | POST /api/user/email/change | {user_id}");

    if request.new_email.inner.is_empty() {
        return Err((Status::BadRequest, "New email cannot be empty.".to_string()));
    }

    if let Ok(_user_id) = database::get_user_id(db_pool, request.new_email.clone()).await {
        return Err((
            Status::Conflict,
            "An account with this new email already exists.".to_string(),
        ));
    }

    let old_email = database::get_user_info(db_pool, user_id.clone())
        .await
        .map_err(expose_internal_error)?
        .email;

    let email_verification_token = EmailVerificationToken {
        inner: auth::generate_secret(),
    };
    let email_verification_token_hash = auth::create_secret_hash(
        &email_verification_token.inner,
        SecretHashOptions {
            use_specific_salt: Some(app_user_secrets.inner.clone()),
        },
    )
    .map_err(expose_internal_error)?;

    let email_verification_token_expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

    database::update_user_email_change_fields(
        db_pool,
        &user_id,
        request.new_email.clone(),
        email_verification_token_hash,
        email_verification_token_expires_at,
    )
    .await
    .map_err(expose_internal_error)?;

    let new_email = request.new_email.clone();
    email::send_email_change_confirmation_link_to_new_email(
        smtp_config,
        &new_email.clone(),
        &email_verification_token,
    )
    .await
    .map_err(expose_internal_error)?;

    log::info!("# | post_api_user_me_email_change | Email change confirmation sent to {new_email}");

    let _ = email::send_email_change_notification_to_old_email(
        smtp_config,
        &old_email,
        &request.new_email,
    )
    .await
    .inspect_err(|e|log::warn!("# | post_api_user_me_email_change | Failed to send Email change notification sent to {old_email}: {e}"))
    .inspect(|()| log::info!("# | post_api_user_me_email_change | Email change notification sent to {old_email}"));

    Ok(())
}

#[get("/api/user/info")]
pub async fn get_api_user_info(
    db_pool: &State<PgPool>,
    jwt: jwt::Jwt,
) -> HttpResult<Json<UserInfoUI>> {
    let user_id = jwt.user_id().map_err(expose_internal_error)?;
    log::info!("# | GET /api/user/info | {user_id}");
    let user_info = database::get_user_info(db_pool, user_id.clone())
        .await
        .map_err(expose_internal_error)?;
    log::info!("# | GET /api/user/info | {user_id} | user_info");
    Ok(Json(user_info.into()))
}

// NOTE: specta::Type is manually exported in bindings
#[derive(Serialize, Deserialize)]
pub struct UserInfoUI {
    pub id: UserId,
    pub email: Email,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<database::UserInfo> for UserInfoUI {
    fn from(user: database::UserInfo) -> Self {
        let database::UserInfo {
            id,
            email,
            password_hash: _,
            created_at,
            new_email: _,
            email_verification_token_hash: _,
            email_verification_token_expires_at: _,
        } = user;
        Self {
            id,
            email,
            created_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
pub struct DeleteAccountRequest {
    pub password: UserProvidedPassword,
    pub confirmation: String,
}

#[delete("/api/user", data = "<request>")]
pub async fn delete_api_user(
    db_pool: &State<PgPool>,
    smtp_config: &State<SmtpConfig>,
    jwt_app_secret: &State<jwt::ApplicationJwtSecret>,
    shared_updaters: &State<updater::UpdaterManager>,
    jwt: jwt::Jwt,
    request: Json<DeleteAccountRequest>,
) -> HttpResult<()> {
    let user_id = jwt.user_id().map_err(expose_internal_error)?;
    log::info!("# | DELETE /api/user | {user_id}");

    if request.confirmation != "delete" {
        return Err((
            Status::BadRequest,
            "Confirmation string must be exactly 'delete'".to_string(),
        ));
    }

    let user_info = database::get_user_info(db_pool, user_id.clone())
        .await
        .map_err(expose_internal_error)?;

    let _token =
        auth::verify_password_and_create_token(&request.password, &user_info, jwt_app_secret)
            .map_err(|_| (Status::Unauthorized, "Invalid password".to_string()))?;

    // Stop updater tasks (log errors but don't fail if it doesn't work)
    if let Err(e) = shared_updaters.stop_updater(&user_id) {
        log::warn!("# | DELETE /api/user | {user_id} | Failed to stop updater: {e}");
    }

    // cascading deletion from database
    database::delete_user(db_pool, &user_id)
        .await
        .map_err(expose_internal_error)?;

    if let Err(e) = email::send_account_deletion_notification(smtp_config, &user_info.email).await {
        log::warn!(
            "# | DELETE /api/user | {user_id} | Failed to send deletion notification email: {e}"
        );
    }

    log::info!("# | DELETE /api/user | {user_id} | Account deleted successfully");

    Ok(())
}
