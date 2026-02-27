use crate::platforms::vrchat_api::VRChatAuthResponse;
use crate::platforms::vrchat_auth_types::{
    Cookies, TwoFactorAuthCode, TwoFactorAuthMethod, TwoFactorCodeRequiredResponse,
    VRChatCredentials, VRChatCredentialsWithCookie, VRChatCredentialsWithTwoFactorAuth,
    VRChatUserId,
};
use crate::users;

use anyhow::{Result, anyhow};
use base64::prelude::*;
use either::Either;
use std::sync::Arc;
use vrchatapi::{
    apis::{authentication_api, configuration::Configuration as VrcConfig},
    models as vrc,
};

const VRCHAT_UPDATER_USER_AGENT: &str = concat!(
    "PluralSync/",
    env!("CARGO_PKG_VERSION"),
    " ",
    env!("USER_AGENT_EMAIL")
);
/* Called in updater. Cookie is only validated, no new cookie is created. */
pub async fn authenticate_vrchat_with_cookie(
    config: &users::UserConfigForUpdater,
) -> Result<(VrcConfig, Cookies, VRChatUserId)> {
    log::info!("# | authenticate_vrchat_with_cookie | {}", config.user_id);

    let creds = VRChatCredentialsWithCookie::from_config(config);

    let (vrchat_config, cookies) =
        new_vrchat_config_with_basic_auth_and_optional_cookie(Either::Right(&creds))?;

    let () = match authentication_api::get_current_user(&vrchat_config).await? {
        vrc::EitherUserOrTwoFactor::CurrentUser(_me) => Ok(()),
        vrc::EitherUserOrTwoFactor::RequiresTwoFactorAuth(_) => {
            Err(anyhow!("authenticate_vrchat_with_cookie: Login failed"))
        }
    }?;

    log::info!(
        "# | authenticate_vrchat_with_cookie | {} | cookie_valid",
        config.user_id
    );

    let vrc_user_id = get_vrchat_user_id(config, &vrchat_config).await?;

    log::info!(
        "# | authenticate_vrchat_with_cookie | {} | cookie_valid | vrc_user_id {vrc_user_id}",
        config.user_id
    );

    Ok((vrchat_config, cookies, vrc_user_id))
}

pub async fn authenticate_vrchat_for_new_cookie(
    creds: &VRChatCredentials,
) -> Result<VRChatAuthResponse> {
    let (vrchat_config, cookies) =
        new_vrchat_config_with_basic_auth_and_optional_cookie(Either::Left(creds))?;

    log::info!("# | authenticate_vrchat_for_new_cookie | {creds}");

    let result = match authentication_api::get_current_user(&vrchat_config).await? {
        // User doesn't need 2fa
        vrc::EitherUserOrTwoFactor::CurrentUser(_me) => {
            let cookie = serialize_cookie_store(&cookies)?;
            let creds_with_cookie = VRChatCredentialsWithCookie::from(creds.clone(), cookie);
            Either::Left(creds_with_cookie)
        }

        vrc::EitherUserOrTwoFactor::RequiresTwoFactorAuth(requires_auth) => {
            let method = TwoFactorAuthMethod::from(&requires_auth);
            let tmp_cookie = serialize_cookie_store(&cookies)?;
            Either::Right(TwoFactorCodeRequiredResponse { method, tmp_cookie })
        }
    };

    log::info!("# | authenticate_vrchat_for_new_cookie | {creds} | {result}");

    Ok(result)
}

pub async fn authenticate_vrchat_for_new_cookie_with_2fa(
    creds_with_tfa: &VRChatCredentialsWithTwoFactorAuth,
) -> Result<VRChatCredentialsWithCookie> {
    let creds_with_tmp_cookie = VRChatCredentialsWithCookie {
        creds: creds_with_tfa.creds.clone(),
        cookie: creds_with_tfa.tmp_cookie.clone(),
    };

    log::info!("# | authenticate_vrchat_for_new_cookie_with_2fa | {creds_with_tmp_cookie}");

    let (vrchat_config, cookies) = new_vrchat_config_with_basic_auth_and_optional_cookie(
        Either::Right(&creds_with_tmp_cookie),
    )?;

    let () =
        vrchat_verify_2fa(&creds_with_tfa.method, &creds_with_tfa.code, &vrchat_config).await?;

    let cookie = serialize_cookie_store(&cookies)?;

    log::info!(
        "# | authenticate_vrchat_for_new_cookie_with_2fa | {creds_with_tmp_cookie} | verified"
    );

    Ok(VRChatCredentialsWithCookie::from(
        creds_with_tfa.creds.clone(),
        cookie,
    ))
}

async fn vrchat_verify_2fa(
    method: &TwoFactorAuthMethod,
    auth_code: &TwoFactorAuthCode,
    vrchat_config: &VrcConfig,
) -> Result<()> {
    match method {
        TwoFactorAuthMethod::TwoFactorAuthMethodEmail => {
            authentication_api::verify2_fa_email_code(
                vrchat_config,
                vrc::TwoFactorEmailCode::new(auth_code.clone().into()),
            )
            .await?;
        }
        TwoFactorAuthMethod::TwoFactorAuthMethodApp => {
            authentication_api::verify2_fa(
                vrchat_config,
                vrc::TwoFactorAuthCode::new(auth_code.clone().into()),
            )
            .await?;
        }
    }

    Ok(())
}

fn new_vrchat_config_with_basic_auth_and_optional_cookie(
    creds: Either<&VRChatCredentials, &VRChatCredentialsWithCookie>,
) -> Result<(VrcConfig, Cookies)> {
    let my_cookie_store = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(
        reqwest_cookie_store::CookieStore::new(None),
    ));

    let username = creds.either(|c| &c.username, |c| &c.creds.username);
    let password = creds.either(|c| &c.password, |c| &c.creds.password);
    let cookie_str = creds.right().map(|c| &c.cookie);

    if let Some(serialized_cookies) = cookie_str {
        let cookies = deserialize_cookie_store(serialized_cookies)?;
        let mut cs = my_cookie_store.lock().map_err(|e| anyhow!(e.to_string()))?;
        *cs = cookies;
    }

    let vrchat_config = VrcConfig {
        user_agent: Some(VRCHAT_UPDATER_USER_AGENT.to_string()),
        basic_auth: Some((username.clone(), Some(password.clone()))),
        client: reqwest::Client::builder()
            .cookie_provider(my_cookie_store.clone())
            .build()?,
        ..Default::default()
    };

    Ok((vrchat_config, my_cookie_store))
}

pub fn serialize_cookie_store(cookies: &Cookies) -> Result<String> {
    let a = cookies.lock().map_err(|e| anyhow!(e.to_string()))?.clone();
    let b = serde_json::to_string(&a)?;
    Ok(BASE64_STANDARD.encode(b))
}

fn deserialize_cookie_store(s: &String) -> Result<reqwest_cookie_store::CookieStore> {
    let base64_decoded: String = BASE64_STANDARD.decode(s)?.try_into()?;
    let cookies = serde_json::from_str(base64_decoded.as_str())?;
    Ok(cookies)
}

async fn get_vrchat_user_id(
    config: &users::UserConfigForUpdater,
    vrchat_config: &VrcConfig,
) -> Result<VRChatUserId> {
    match authentication_api::get_current_user(vrchat_config).await? {
        vrc::EitherUserOrTwoFactor::CurrentUser(user) => Ok(VRChatUserId { inner: user.id }),
        vrc::EitherUserOrTwoFactor::RequiresTwoFactorAuth(_) => Err(anyhow!(
            "get_vrchat_user_id: Cookie invalid for user {}",
            config.vrchat_username.secret
        )),
    }
}
