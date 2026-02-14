use crate::database;
use crate::meta_api::HttpResult;
use crate::meta_api::expose_internal_error;
use crate::metrics::{PASSWORD_RESET_FAILURE_TOTAL, PASSWORD_RESET_SUCCESS_TOTAL};
use crate::setup::SmtpConfig;
use crate::users::auth;
use crate::users::email;
use crate::users::jwt;
use crate::users::model::UserId;
use anyhow::anyhow;
use chrono::Utc;
use pluralsync_base::users::Email;
use pluralsync_base::users::JwtString;
use pluralsync_base::users::PasswordResetToken;
use pluralsync_base::users::UserLoginCredentials;
use pluralsync_base::users::UserProvidedPassword;
use rocket::http;
use rocket::{State, get, post, serde::json::Json};
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;

#[post("/api/user/register", data = "<credentials>")]
pub async fn post_api_user_register(
    db_pool: &State<PgPool>,
    credentials: Json<UserLoginCredentials>,
) -> HttpResult<()> {
    log::info!("# | POST /api/user/register | {}", credentials.email);

    if credentials.is_empty_and_thus_invalid() {
        return Err(anyhow!("Email/Passsword cannot be empty.")).map_err(expose_internal_error)?;
    }

    let pwh =
        auth::create_secret_hash(&credentials.password.inner).map_err(expose_internal_error)?;

    let () = database::create_user(db_pool, credentials.email.clone(), pwh)
        .await
        .map_err(expose_internal_error)?;

    log::info!(
        "# | POST /api/user/register | {} | user created.",
        credentials.email
    );

    Ok(())
}

#[post("/api/user/login", data = "<credentials>")]
pub async fn post_api_user_login(
    db_pool: &State<PgPool>,
    jwt_app_secret: &State<jwt::ApplicationJwtSecret>,
    credentials: Json<UserLoginCredentials>,
) -> Result<Json<JwtString>, (http::Status, String)> {
    log::info!("# | POST /api/user/login | {}", credentials.email);

    if credentials.is_empty_and_thus_invalid() {
        return Err(anyhow!("Email/Passsword cannot be empty.")).map_err(expose_internal_error)?;
    }

    let user_id = database::get_user_id(db_pool, credentials.email.clone())
        .await
        .map_err(|e| (http::Status::Forbidden, e.to_string()))?;

    log::info!(
        "# | POST /api/user/login | {} | {user_id}",
        &credentials.email
    );

    let user_info = database::get_user_info(db_pool, user_id.clone())
        .await
        .map_err(|e| (http::Status::InternalServerError, e.to_string()))?;

    log::info!(
        "# | POST /api/user/login | {} | {user_id} | user_info",
        &credentials.email
    );

    let jwt_string =
        auth::verify_password_and_create_token(&credentials.password, &user_info, jwt_app_secret)
            .map_err(|e| (http::Status::Forbidden, e.to_string()))?;

    log::info!(
        "# | POST /api/user/login | {} | {user_id} | user_info | jwt created",
        &credentials.email
    );

    Ok(Json(jwt_string))
}

#[post("/api/auth/forgot-password", data = "<request>")]
pub async fn post_api_auth_forgot_password(
    db_pool: &State<PgPool>,
    smtp_config: &State<SmtpConfig>,
    request: Json<ForgotPasswordRequest>,
) -> HttpResult<()> {
    log::info!("# | POST /api/auth/forgot-password | {}", request.email);

    match create_password_reset_request_and_send_email(db_pool, smtp_config, &request).await {
        Ok(()) => {
            log::info!(
                "# | POST /api/auth/forgot-password | {} | reset email initiated.",
                request.email
            );
        }
        Err(e) => {
            log::warn!(
                "# | POST /api/auth/forgot-password | {} | failed or user not found: {:?}",
                request.email,
                e
            );
        }
    }

    // Always return 200 OK to prevent email enumeration
    Ok(())
}

async fn create_password_reset_request_and_send_email(
    db_pool: &State<PgPool>,
    smtp_config: &State<SmtpConfig>,
    request: &Json<ForgotPasswordRequest>,
) -> anyhow::Result<()> {
    let token_secret = PasswordResetToken {
        inner: auth::generate_secret(),
    };
    let token_hash = auth::create_secret_hash(&token_secret.inner)?;

    let user_id = database::get_user_id(db_pool, request.email.clone()).await?;

    let expiration = Utc::now() + chrono::Duration::hours(1);

    database::create_password_reset_request(db_pool, &user_id, &token_hash, &expiration).await?;

    // Asynchronously send the reset email
    let email = request.email.clone();
    let smtp_config = smtp_config.inner().clone();
    tokio::spawn(async move {
        email::send_reset_email(&smtp_config, &email, &token_secret).await
            .map(|()| log::info!("# | create_password_reset_request_and_send_email | Email sent to {email}"))
            .map_err(|e| log::warn!("# | create_password_reset_request_and_send_email | Failed to send email to {email}: {e:?}"))
    });

    Ok(())
}

#[post("/api/auth/reset-password", data = "<request>")]
pub async fn post_api_auth_reset_password(
    db_pool: &State<PgPool>,
    request: Json<ResetPasswordAttempt>,
) -> HttpResult<()> {
    log::info!("# | POST /api/auth/reset-password");

    let token_hash = auth::create_secret_hash(&request.token.inner).map_err(|_| {
        (
            http::Status::BadRequest,
            "Invalid or expired token".to_string(),
        )
    })?;

    let user_id = database::verify_password_reset_request_matches(db_pool, &token_hash)
        .await
        .map_err(|_| {
            (
                http::Status::BadRequest,
                "Invalid or expired token".to_string(),
            )
        })?;

    log::debug!("# | POST /api/auth/reset-password | Verified password reset request {user_id}");

    let new_password_hash =
        auth::create_secret_hash(&request.new_password.inner).map_err(expose_internal_error)?;

    let update_result = database::update_user_password(db_pool, &user_id, &new_password_hash).await;

    if update_result.is_ok() {
        PASSWORD_RESET_SUCCESS_TOTAL
            .with_label_values(&["post_api_auth_reset_password"])
            .inc();
    } else {
        PASSWORD_RESET_FAILURE_TOTAL
            .with_label_values(&["post_api_auth_reset_password"])
            .inc();
    }

    update_result.map_err(expose_internal_error)?;

    database::delete_password_reset_request(db_pool, &user_id)
        .await
        .map_err(expose_internal_error)?;

    log::info!(
        "# | POST /api/auth/reset-password | Verified password reset request {user_id} | Password reset success",
    );

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
        } = user;
        Self {
            id,
            email,
            created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: Email,
}

#[derive(Deserialize)]
pub struct ResetPasswordAttempt {
    pub token: PasswordResetToken,
    pub new_password: UserProvidedPassword,
}
