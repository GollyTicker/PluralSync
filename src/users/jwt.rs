use crate::users::model::UserId;
use anyhow::{Result, anyhow};
use chrono::Duration;
use jsonwebtoken::{EncodingKey, Header, encode};
use pluralsync_base::{clock, users::JwtString};
use rocket::{
    Request, State,
    http::Status,
    request::{FromRequest, Outcome},
    response,
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ApplicationJwtSecret {
    pub inner: String,
}

#[derive(Serialize, Deserialize)]
pub struct Jwt {
    pub claims: Claims,
}

impl Jwt {
    pub fn user_id(&self) -> Result<UserId> {
        self.claims.user_id()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    /// `PluralSync` `user_id`
    pub sub: String,
    pub exp: usize,
}

impl Claims {
    pub fn user_id(&self) -> Result<UserId> {
        Ok(UserId {
            inner: self.sub.clone().try_into()?,
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Jwt {
    type Error = rocket::response::Debug<anyhow::Error>;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        fn no_jwt_provided_outcome() -> Outcome<Jwt, response::Debug<anyhow::Error>> {
            Outcome::Error((
                Status::Unauthorized,
                response::Debug::from(anyhow!("from_request: No Jwt provided")),
            ))
        }

        fn verify_jwt_and_handle_result(
            auth_header_value: &str,
            jwt_secret: &ApplicationJwtSecret,
        ) -> Outcome<Jwt, response::Debug<anyhow::Error>> {
            let token = JwtString {
                inner: auth_header_value
                    .trim_start_matches("Bearer")
                    .trim()
                    .to_owned(),
            };
            log::debug!("# | jwt verification | {token}");
            match verify_jwt(&token, jwt_secret) {
                Ok((claims, user_id)) => {
                    log::debug!("# | jwt verification | {token} | verified | {user_id}");
                    Outcome::Success(Jwt { claims })
                }
                Err(err) => {
                    log::warn!("# | jwt verification | {token} | failed | {err}");
                    Outcome::Error((
                        Status::Forbidden,
                        response::Debug(anyhow!("Token verification failed")),
                    ))
                }
            }
        }

        let jwt_secret = req
            .guard::<&State<ApplicationJwtSecret>>()
            .await
            .map_error(|(err_status, ())| (err_status, response::Debug(anyhow!(err_status))));

        let auth_header_value = req.headers().get_one("authorization");

        auth_header_value.map_or_else(no_jwt_provided_outcome, |auth_header_value| {
            jwt_secret
                .and_then(|jwt_secret| verify_jwt_and_handle_result(auth_header_value, jwt_secret))
        })
    }
}

const JWT_VALID_HOURS: i64 = 15;

pub fn create_token(user_id: &UserId, jwt_secret: &ApplicationJwtSecret) -> Result<JwtString> {
    let expiration: usize = clock::now()
        .checked_add_signed(Duration::hours(JWT_VALID_HOURS))
        .ok_or_else(|| anyhow!("create_token: invalid timestamp"))?
        .timestamp()
        .try_into()?;

    let claims = Claims {
        sub: user_id.inner.to_string(),
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.inner.as_bytes()),
    )?;

    Ok(JwtString { inner: token })
}

pub fn verify_jwt(
    token: &JwtString,
    jwt_secret: &ApplicationJwtSecret,
) -> Result<(Claims, UserId)> {
    let token_data = jsonwebtoken::decode::<Claims>(
        &token.inner,
        &jsonwebtoken::DecodingKey::from_secret(jwt_secret.inner.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )?;

    let user_id_str: &str = &token_data.claims.sub.clone();

    Ok((token_data.claims, user_id_str.try_into()?))
}
