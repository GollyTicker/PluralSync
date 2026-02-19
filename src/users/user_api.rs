use crate::database;
use crate::meta_api::HttpResult;
use crate::meta_api::expose_internal_error;
use crate::metrics::{PASSWORD_RESET_FAILURE_TOTAL, PASSWORD_RESET_SUCCESS_TOTAL};
use crate::setup::SmtpConfig;
use crate::users::SecretHashOptions;
use crate::users::auth;
use crate::users::email;
use crate::users::jwt;
use crate::users::model::UserId;
use anyhow::anyhow;
use chrono::Utc;
use pluralsync_base::users::Email;
use pluralsync_base::users::EmailVerificationToken;
use pluralsync_base::users::JwtString;
use pluralsync_base::users::PasswordResetToken;
use pluralsync_base::users::Secret;
use pluralsync_base::users::UserLoginCredentials;
use pluralsync_base::users::UserProvidedPassword;
use rocket::http;
use rocket::http::Status;
use rocket::{State, get, post, serde::json::Json};
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize, Deserialize, specta::Type)]
pub struct EmailVerificationResponse {
    pub message: String,
}

#[post("/api/user/email/verify/<token>")]
pub async fn post_api_user_email_verify(
    db_pool: &State<PgPool>,
    app_user_secrets: &State<database::ApplicationUserSecrets>,
    token: String,
) -> HttpResult<Json<EmailVerificationResponse>> {
    log::info!("# | POST /api/user/email/verify/{token}");

    let email_verification_token = EmailVerificationToken {
        inner: Secret { inner: token },
    };
    let email_verification_token_hash = auth::create_secret_hash(
        &email_verification_token.inner,
        SecretHashOptions {
            use_specific_salt: Some(app_user_secrets.inner.clone()),
        },
    )
    .map_err(expose_internal_error)?;

    // Try to verify as an email change of an existing user
    let existing_user = database::find_user_id_by_email_verification_token_hash(
        db_pool,
        &email_verification_token_hash,
    )
    .await;
    if let Ok(Some(user_id)) = existing_user
        && let Ok(user_info) = database::get_user_info(db_pool, user_id.clone()).await
        && let Some(new_email) = user_info.new_email
        && let Some(expires_at) = user_info.email_verification_token_expires_at
        && expires_at > Utc::now()
    {
        database::set_new_verified_email(db_pool, &user_id, new_email.clone())
            .await
            .map_err(expose_internal_error)?;
        log::info!("# | POST /api/user/email/verify | Email changed for {user_id} to {new_email}");
        return Ok(Json(EmailVerificationResponse {
            message: "Your email address has been successfully changed.".to_string(),
        }));
    }

    // Otherwise, try to verify as an initial registration
    let temporary_user_result =
        database::find_temporary_user_by_token_hash(db_pool, &email_verification_token_hash).await;
    if let Ok(Some(temporary_user)) = temporary_user_result
        && temporary_user.email_verification_token_expires_at > Utc::now()
    {
        database::create_user(
            db_pool,
            temporary_user.email.clone(),
            temporary_user.password_hash,
        )
        .await
        .map_err(expose_internal_error)?;
        log::info!(
            "# | POST /api/user/email/verify | Initial email verified for {}. Account created",
            temporary_user.email
        );
        return Ok(Json(EmailVerificationResponse {
            message: "Your account has been successfully activated. You can now log in."
                .to_string(),
        }));
    }

    Err((Status::BadRequest, "Token invalid or expired.".to_string()))
}

#[post("/api/user/register", data = "<credentials>")]
pub async fn post_api_user_register(
    db_pool: &State<PgPool>,
    smtp_config: &State<SmtpConfig>,
    app_user_secrets: &State<database::ApplicationUserSecrets>,
    credentials: Json<UserLoginCredentials>,
) -> HttpResult<()> {
    let email = credentials.email.clone();
    log::info!("# | POST /api/user/register | {email}");

    if credentials.is_empty_and_thus_invalid() {
        return Err((
            Status::BadRequest,
            anyhow!("Email/Password cannot be empty.").to_string(),
        ));
    }

    if let Ok(_user_id) = database::get_user_id(db_pool, email.clone()).await {
        return Err((
            Status::Conflict,
            anyhow!("This email is already being used.").to_string(),
        ));
    }

    let password_hash = auth::create_secret_hash(
        &credentials.password.inner,
        SecretHashOptions {
            use_specific_salt: None,
        },
    )
    .map_err(expose_internal_error)?;

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

    let email_verification_token_expires_at = Utc::now() + chrono::Duration::hours(1);

    database::create_or_update_temporary_user(
        db_pool,
        email.clone(),
        password_hash,
        email_verification_token_hash,
        email_verification_token_expires_at,
    )
    .await
    .map_err(expose_internal_error)?;

    email::send_verification_email(smtp_config, &email, &email_verification_token)
        .await
        .map_err(expose_internal_error)?;

    log::info!("# | post_api_user_register | Email verification sent to {email}");

    log::info!(
        "# | POST /api/user/register | {email} | temporary user created and verification email sent."
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

#[post("/api/user/forgot-password", data = "<request>")]
pub async fn post_api_auth_forgot_password(
    db_pool: &State<PgPool>,
    smtp_config: &State<SmtpConfig>,
    app_user_secrets: &State<database::ApplicationUserSecrets>,
    request: Json<ForgotPasswordRequest>,
) -> HttpResult<()> {
    log::info!("# | POST /api/user/forgot-password | {}", request.email);

    match create_password_reset_request_and_send_email(
        db_pool,
        smtp_config,
        app_user_secrets,
        &request,
    )
    .await
    {
        Ok(()) => {
            log::info!(
                "# | POST /api/user/forgot-password | {} | reset email initiated.",
                request.email
            );
        }
        Err(e) => {
            log::warn!(
                "# | POST /api/user/forgot-password | {} | failed or user not found: {:?}",
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
    app_user_secrets: &database::ApplicationUserSecrets,
    request: &Json<ForgotPasswordRequest>,
) -> anyhow::Result<()> {
    let token_secret = PasswordResetToken {
        inner: auth::generate_secret(),
    };
    let token_hash = auth::create_secret_hash(
        &token_secret.inner,
        SecretHashOptions {
            use_specific_salt: Some(app_user_secrets.inner.clone()),
        },
    )?;

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

#[post("/api/user/reset-password", data = "<request>")]
pub async fn post_api_auth_reset_password(
    db_pool: &State<PgPool>,
    app_user_secrets: &State<database::ApplicationUserSecrets>,
    request: Json<ResetPasswordAttempt>,
) -> HttpResult<()> {
    log::info!("# | POST /api/user/reset-password");

    let token_hash = auth::create_secret_hash(
        &request.token.inner,
        SecretHashOptions {
            use_specific_salt: Some(app_user_secrets.inner.clone()),
        },
    )
    .map_err(|_| {
        (
            http::Status::BadRequest,
            "Invalid or expired token".to_string(),
        )
    })?;

    let user_id = database::verify_password_reset_request_matches(db_pool, &token_hash)
        .await
        .map_err(|e| {
            log::warn!("# | POST /api/user/reset-password | Failed to verify: {e}");
            (
                http::Status::BadRequest,
                "Invalid or expired token".to_string(),
            )
        })?;

    log::debug!("# | POST /api/user/reset-password | Verified password reset request {user_id}");

    let new_password_hash = auth::create_secret_hash(
        &request.new_password.inner,
        SecretHashOptions {
            use_specific_salt: None,
        },
    )
    .map_err(expose_internal_error)?;

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
        "# | POST /api/user/reset-password | Verified password reset request {user_id} | Password reset success",
    );

    Ok(())
}

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

    let email_verification_token_expires_at = Utc::now() + chrono::Duration::hours(1);

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

#[derive(Debug, Deserialize, specta::Type)]
pub struct ForgotPasswordRequest {
    pub email: Email,
}

#[derive(Deserialize, specta::Type)]
pub struct ResetPasswordAttempt {
    pub token: PasswordResetToken,
    pub new_password: UserProvidedPassword,
}
