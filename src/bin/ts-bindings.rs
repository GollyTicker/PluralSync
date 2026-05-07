use anyhow::Result;
use pluralsync::{
    database::Decrypted,
    history::HistoryEntry,
    platforms::{
        TwoFactorAuthCode, TwoFactorAuthMethod, TwoFactorCodeRequiredResponse, VRChatCredentials,
        VRChatCredentialsWithCookie, VRChatCredentialsWithTwoFactorAuth,
        webview_api::FrontingStatusWithExclusions,
    },
    updater::Platform,
    users::{
        DiscordRichPresenceUrl, PrivacyFineGrained, UserId,
        auth_endpoints::{
            EmailVerificationResponse, ForgotPasswordRequest, ResetPasswordAttempt,
        },
        user_endpoints::{ChangeEmailRequest, DeleteAccountRequest},
    },
    plurality::{ExcludedFronter, ExclusionReason, FilteredFronter, FilteredFronters, Fronter},
};
use pluralsync_base::{
    meta::{
        CANONICAL_PLURALSYNC_BASE_URL, PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL,
        PluralSyncVariantInfo,
    },
    users::{
        Email, EmailVerificationToken, JwtString, PasswordResetToken, Secret, UserLoginCredentials,
        UserProvidedPassword,
    },
};
use specta::ts::{ExportConfiguration, export};
use std::fs;

const DESTINATION: &str = "./frontend/src/pluralsync.bindings.ts";

fn main() -> Result<()> {
    println!("Exporting to {DESTINATION}...");
    let conf = &ExportConfiguration::default();
    let defs = [
        export::<UserId>(conf)?,
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
    enable_from_sp?: boolean;
    enable_from_websocket?: boolean;
    enable_to_pluralkit?: boolean;
    enable_from_pluralkit?: boolean;
    simply_plural_token?: Decrypted;
    discord_status_message_token?: Decrypted;
    vrchat_username?: Decrypted;
    vrchat_password?: Decrypted;
    vrchat_cookie?: Decrypted;
    pluralkit_token?: Decrypted;
    from_pluralkit_webhook_signing_token?: Decrypted;
    history_limit?: number;
    history_truncate_after_days?: number;
    fronter_channel_wait_increment?: number;
    discord_rich_presence_url?: DiscordRichPresenceUrl;
    discord_rich_presence_url_custom?: string;
    from_pluralkit_prefer_displayname?: boolean;
    from_pluralkit_respect_member_visibility?: boolean;
    from_pluralkit_respect_field_visibility?: boolean;
}".to_owned(),
        export::<PrivacyFineGrained>(conf)?,
        export::<DiscordRichPresenceUrl>(conf)?,
        export::<JwtString>(conf)?,
        export::<Platform>(conf)?,
        "export type UpdaterStatus = \"Disabled\" | \"Running\" | { \"Error\": string } | \"Starting\"".to_owned(),
        "export type UserUpdatersStatuses = { [p in Platform]?: UpdaterStatus }".to_owned(),
        export::<FrontingStatusWithExclusions>(conf)?,
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
        export::<EmailVerificationToken>(conf)?,
        export::<ChangeEmailRequest>(conf)?,
        export::<EmailVerificationResponse>(conf)?,
        export::<DeleteAccountRequest>(conf)?,
        "export type UserInfoUI = { id: UserId, email: { inner: string }, created_at: string }".to_owned(),
        export::<HistoryEntry>(conf)?,
        export::<Fronter>(conf)?,
        export::<ExcludedFronter>(conf)?,
        export::<ExclusionReason>(conf)?,
        export::<FilteredFronter>(conf)?,
        export::<FilteredFronters>(conf)?,
    ];
    fs::write(DESTINATION, defs.map(|s| s + ";").join("\n"))?;
    println!("Done.");
    Ok(())
}
