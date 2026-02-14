use crate::database;
use crate::meta_api::HttpResult;
use crate::meta_api::expose_internal_error;
use crate::users::auth;
use crate::users::jwt;
use crate::users::model::UserId;
use anyhow::anyhow;
use pluralsync_base::users::Email;
use pluralsync_base::users::JwtString;
use pluralsync_base::users::UserLoginCredentials;
use rocket::http;
use rocket::{State, serde::json::Json};
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

    let pwh = auth::create_password_hash(&credentials.password).map_err(expose_internal_error)?;

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
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: pluralsync_base::users::UserProvidedPassword,
}
