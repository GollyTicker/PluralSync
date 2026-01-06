use std::{fmt::Display, sync::Arc};

use crate::users;

use serde::{Deserialize, Serialize};
use specta;
use strum_macros;
use vrchatapi::models::{RequiresTwoFactorAuth, TwoFactorAuthType};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct VRChatUserId {
    pub inner: String,
}

pub type Cookies = Arc<reqwest_cookie_store::CookieStoreMutex>;

#[derive(Clone, Deserialize, Serialize, Debug, specta::Type)]
pub struct VRChatCredentials {
    pub username: String,
    pub password: String,
}

impl Display for VRChatCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VRChatCredentials({}, <password>)", self.username)
    }
}

#[derive(Clone, Serialize, Debug, specta::Type)]
pub struct VRChatCredentialsWithCookie {
    pub creds: VRChatCredentials,
    pub cookie: String,
}

impl Display for VRChatCredentialsWithCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VRChatCredentialsWithCookie({}, <password>, <cookie>)",
            self.creds.username
        )
    }
}

impl VRChatCredentialsWithCookie {
    #[must_use]
    pub fn from_config(config: &users::UserConfigForUpdater) -> Self {
        Self::from_strings(
            config.vrchat_username.secret.clone(),
            config.vrchat_password.secret.clone(),
            config.vrchat_cookie.secret.clone(),
        )
    }

    #[must_use]
    pub fn from(creds: VRChatCredentials, cookie: String) -> Self {
        Self::from_strings(creds.username, creds.password, cookie)
    }

    const fn from_strings(username: String, password: String, cookie: String) -> Self {
        Self {
            creds: VRChatCredentials { username, password },
            cookie,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TwoFactorCodeRequiredResponse {
    pub method: TwoFactorAuthMethod,
    pub tmp_cookie: String,
}

impl Display for TwoFactorCodeRequiredResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TwoFactorCodeRequiredResponse({}, <tmp_cookie>)",
            self.method
        )
    }
}

#[derive(Clone, Serialize, Deserialize, strum_macros::Display, Debug, specta::Type)]
pub enum TwoFactorAuthMethod {
    TwoFactorAuthMethodEmail,
    TwoFactorAuthMethodApp,
}

impl TwoFactorAuthMethod {
    #[must_use]
    pub fn from(requires_2fa_auth: &RequiresTwoFactorAuth) -> Self {
        let is_email_2fa = requires_2fa_auth
            .requires_two_factor_auth
            .contains(&TwoFactorAuthType::EmailOtp);

        if is_email_2fa {
            Self::TwoFactorAuthMethodEmail
        } else {
            Self::TwoFactorAuthMethodApp
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, specta::Type)]
pub struct TwoFactorAuthCode {
    inner: String,
}

impl Display for TwoFactorAuthCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TwoFactorAuthCode({})", self.inner)
    }
}

impl From<TwoFactorAuthCode> for String {
    fn from(val: TwoFactorAuthCode) -> Self {
        val.inner
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, specta::Type)]
pub struct VRChatCredentialsWithTwoFactorAuth {
    pub creds: VRChatCredentials,
    pub method: TwoFactorAuthMethod,
    pub code: TwoFactorAuthCode,
    pub tmp_cookie: String,
}

impl Display for VRChatCredentialsWithTwoFactorAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VRChatCredentialsWithTwoFactorAuth({}, {}, {}, <tmp_cookie>)",
            self.creds, self.method, self.code
        )
    }
}
