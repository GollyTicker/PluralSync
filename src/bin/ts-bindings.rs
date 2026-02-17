use anyhow::Result;
use pluralsync::{
    database::Decrypted,
    platforms::{
        TwoFactorAuthCode, TwoFactorAuthMethod, TwoFactorCodeRequiredResponse, VRChatCredentials,
        VRChatCredentialsWithCookie, VRChatCredentialsWithTwoFactorAuth,
        webview_api::GenericFrontingStatus,
    },
    updater::Platform,
    users::{PrivacyFineGrained, user_api::{ForgotPasswordRequest, ResetPasswordAttempt}},
};
use pluralsync_base::{
    meta::{CANONICAL_PLURALSYNC_BASE_URL, PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL, PluralSyncVariantInfo},
    users::{Email, JwtString, PasswordResetToken, Secret, UserLoginCredentials, UserProvidedPassword},
};
use specta::ts::{ExportConfiguration, export};
use std::fs;

const DESTINATION: &str = "./frontend/src/pluralsync.bindings.ts";

fn main() -> Result<()> {
    println!("Exporting to {DESTINATION}...");
    let conf = &ExportConfiguration::default();
    let defs = [
        export::<Email>(conf)?,
        export::<UserProvidedPassword>(conf)?,
        export::<Secret>(conf)?,
        export::<UserLoginCredentials>(conf)?,
        export::<Decrypted>(conf)?,
        export::<PluralSyncVariantInfo>(conf)?,
        format!("export const CANONICAL_PLURALSYNC_BASE_URL: string = \"{CANONICAL_PLURALSYNC_BASE_URL}\""),
        format!("export const PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL: string = \"{PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL}\""),
"export type UserConfigDbEntries = {
    website_system_name?: string;
    website_url_name?: string;
    status_prefix?: string;
    status_no_fronts?: string;
    status_truncate_names_to?: number;
    privacy_fine_grained?: PrivacyFineGrained;
    privacy_fine_grained_buckets?: string[];
    show_members_non_archived?: boolean;
    show_members_archived?: boolean;
    show_custom_fronts?: boolean;
    respect_front_notifications_disabled?: boolean;
    enable_website?: boolean;
    enable_discord?: boolean;
    enable_discord_status_message?: boolean;
    enable_vrchat?: boolean;
    enable_to_pluralkit?: boolean;
    simply_plural_token?: Decrypted;
    discord_status_message_token?: Decrypted;
    vrchat_username?: Decrypted;
    vrchat_password?: Decrypted;
    vrchat_cookie?: Decrypted;
    pluralkit_token?: Decrypted;
}".to_owned(),
        export::<PrivacyFineGrained>(conf)?,
        export::<JwtString>(conf)?,
        export::<Platform>(conf)?,
        "export type UpdaterStatus = \"Disabled\" | \"Running\" | { \"Error\": string } | \"Starting\"".to_owned(),
        "export type UserUpdatersStatuses = { [p in Platform]?: UpdaterStatus }".to_owned(),
        export::<GenericFrontingStatus>(conf)?,
        export::<VRChatCredentials>(conf)?,
        export::<VRChatCredentialsWithCookie>(conf)?,
        export::<TwoFactorAuthMethod>(conf)?,
        export::<TwoFactorCodeRequiredResponse>(conf)?,
        export::<TwoFactorAuthCode>(conf)?,
        export::<VRChatCredentialsWithTwoFactorAuth>(conf)?,
        "export type VRChatAuthResponse = { Left: VRChatCredentialsWithCookie } | { Right: TwoFactorCodeRequiredResponse }".to_owned(),
        export::<ResetPasswordAttempt>(conf)?,
        export::<ForgotPasswordRequest>(conf)?,
        export::<PasswordResetToken>(conf)?,
    ];
    fs::write(DESTINATION, defs.map(|s| s + ";").join("\n"))?;
    println!("Done.");
    Ok(())
}
