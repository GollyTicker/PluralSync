export type UserId = { inner: string };
export type Email = { inner: string };
export type UserProvidedPassword = { inner: Secret };
export type Secret = { inner: string };
export type UserLoginCredentials = { email: Email; password: UserProvidedPassword };
export type Decrypted = { secret: string };
export type PluralSyncVariantInfo = { version: string; variant: string; description: string | null; show_in_ui: boolean };
export const CANONICAL_PLURALSYNC_BASE_URL: string = "https://public-test.pluralsync.ayake.net";
export const PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL: string = "https://github.com/GollyTicker/PluralSync/releases";
export type UserConfigDbEntries = {
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
};
export type PrivacyFineGrained = "NoFineGrained" | "ViaFriend" | "ViaPrivacyBuckets";
export type DiscordRichPresenceUrl = "None" | "PluralSyncAboutPage" | "PluralSyncFrontingWebsiteIfDefined" | "CustomUrl";
export type JwtString = { inner: string };
export type Platform = "VRChat" | "Discord" | "DiscordStatusMessage" | "ToPluralKit";
export type UpdaterStatus = "Disabled" | "Running" | { "Error": string } | "Starting";
export type UserUpdatersStatuses = { [p in Platform]?: UpdaterStatus };
export type FrontingStatusWithExclusions = { fronters: Fronter[]; excluded: ExcludedFronter[]; status_text: string };
export type VRChatCredentials = { username: string; password: string };
export type VRChatCredentialsWithCookie = { creds: VRChatCredentials; cookie: string };
export type TwoFactorAuthMethod = "TwoFactorAuthMethodEmail" | "TwoFactorAuthMethodApp";
export type TwoFactorCodeRequiredResponse = { method: TwoFactorAuthMethod; tmp_cookie: string };
export type TwoFactorAuthCode = { inner: string };
export type VRChatCredentialsWithTwoFactorAuth = { creds: VRChatCredentials; method: TwoFactorAuthMethod; code: TwoFactorAuthCode; tmp_cookie: string };
export type VRChatAuthResponse = { Left: VRChatCredentialsWithCookie } | { Right: TwoFactorCodeRequiredResponse };
export type ResetPasswordAttempt = { token: PasswordResetToken; new_password: UserProvidedPassword };
export type ForgotPasswordRequest = { email: Email };
export type PasswordResetToken = { inner: Secret };
export type EmailVerificationToken = { inner: Secret };
export type ChangeEmailRequest = { new_email: Email };
export type EmailVerificationResponse = { message: string };
export type DeleteAccountRequest = { password: UserProvidedPassword; confirmation: string };
export type UserInfoUI = { id: UserId, email: { inner: string }, created_at: string };
export type HistoryEntry = { id: string; user_id: UserId; status_text: string; created_at: string };
/**
 * Generic representation of a fronter from any system source (`SimplyPlural`, `PluralKit`, etc.)
 */
export type Fronter = { fronter_id: string; name: string; pronouns: string | null; avatar_url: string; pluralkit_id: string | null; start_time: string; privacy_buckets: string[] };
/**
 * A fronter that has been excluded along with the reason
 */
export type ExcludedFronter = { fronter: Fronter; reason: ExclusionReason };
/**
 * Reasons why a fronter might be excluded from display
 */
export type ExclusionReason = "FrontNotificationsDisabled" | "ArchivedMemberHidden" | "NonArchivedMemberHidden" | "CustomFrontsDisabled" | "NotInDisplayedPrivacyBuckets" | "MemberPrivacyPrivate";
/**
 * A fronter that has been filtered, either included or excluded with a reason
 */
export type FilteredFronter = { Included: Fronter } | { Excluded: [Fronter, ExclusionReason] };
/**
 * Collection of filtered fronters, separated into included and excluded
 */
export type FilteredFronters = { fronters: Fronter[]; excluded: ExcludedFronter[] };