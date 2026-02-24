use sqlx::{Decode, FromRow, Postgres, error::BoxDynError, postgres};

use crate::{database::secrets, metrics::SHOULDNT_HAPPEN_BUT_IT_DID, users::UserConfigDbEntries};
use anyhow::anyhow;

pub trait ConstraintsType: Clone {}

/// Constraints of the configs from the DB are only valid when
/// * they're validated via config.rs and ONLY THEN put into the DB
/// * read from the DB (since they're valid before putting them in)
#[derive(Clone)]
pub struct ValidConstraints {}

#[derive(Clone, Default, FromRow)]
pub struct InvalidConstraints {}

impl ConstraintsType for ValidConstraints {}
impl ConstraintsType for InvalidConstraints {}

// manual implementation of `Type<Postgres>` because derive doesn't work for non-newtype structs
impl sqlx::Type<Postgres> for ValidConstraints {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <bool as sqlx::Type<Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <bool as sqlx::Type<Postgres>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Postgres> for ValidConstraints {
    fn decode(value: postgres::PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let valid_constraints = <bool as Decode<Postgres>>::decode(value)?;

        if valid_constraints {
            Ok(Self {})
        } else {
            SHOULDNT_HAPPEN_BUT_IT_DID
                .with_label_values(&["decode invalid"])
                .inc();
            Err(anyhow!("decode: ValidConstraints. Implementation bug!").into())
        }
    }
}

// manual implementation of `Type<Postgres>` because derive doesn't work for non-newtype structs
impl sqlx::Type<Postgres> for InvalidConstraints {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <bool as sqlx::Type<Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <bool as sqlx::Type<Postgres>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Postgres> for InvalidConstraints {
    fn decode(value: postgres::PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let valid_constraints = <bool as Decode<Postgres>>::decode(value)?;

        if valid_constraints {
            Err(anyhow!("decode: InvalidConstraints. Implementation bug!").into())
        } else {
            Ok(Self {})
        }
    }
}

pub fn downgrade<Secret: secrets::SecretType, C: ConstraintsType>(
    value: &UserConfigDbEntries<Secret, C>,
) -> UserConfigDbEntries<Secret, InvalidConstraints> {
    UserConfigDbEntries {
        valid_constraints: Some(InvalidConstraints {}),
        website_system_name: value.website_system_name.clone(),
        website_url_name: value.website_url_name.clone(),
        status_prefix: value.status_prefix.clone(),
        status_no_fronts: value.status_no_fronts.clone(),
        status_truncate_names_to: value.status_truncate_names_to,
        show_members_non_archived: value.show_members_non_archived,
        show_members_archived: value.show_members_archived,
        show_custom_fronts: value.show_custom_fronts,
        respect_front_notifications_disabled: value.respect_front_notifications_disabled,
        privacy_fine_grained: value.privacy_fine_grained,
        privacy_fine_grained_buckets: value.privacy_fine_grained_buckets.clone(),
        enable_website: value.enable_website,
        enable_discord: value.enable_discord,
        enable_discord_status_message: value.enable_discord_status_message,
        enable_vrchat: value.enable_vrchat,
        enable_to_pluralkit: value.enable_to_pluralkit,
        simply_plural_token: value.simply_plural_token.clone(),
        discord_status_message_token: value.discord_status_message_token.clone(),
        vrchat_username: value.vrchat_username.clone(),
        vrchat_password: value.vrchat_password.clone(),
        vrchat_cookie: value.vrchat_cookie.clone(),
        pluralkit_token: value.pluralkit_token.clone(),
        history_limit: value.history_limit,
        history_truncate_after_days: value.history_truncate_after_days,
        fronter_channel_wait_increment: value.fronter_channel_wait_increment,
    }
}

pub fn only_use_this_function_to_mark_validation_after_you_have_actually_validated_it<
    Secret: secrets::SecretType,
>(
    value: &UserConfigDbEntries<Secret, InvalidConstraints>,
) -> UserConfigDbEntries<Secret, ValidConstraints> {
    UserConfigDbEntries {
        valid_constraints: Some(ValidConstraints {}),
        website_system_name: value.website_system_name.clone(),
        website_url_name: value.website_url_name.clone(),
        status_prefix: value.status_prefix.clone(),
        status_no_fronts: value.status_no_fronts.clone(),
        status_truncate_names_to: value.status_truncate_names_to,
        show_members_non_archived: value.show_members_non_archived,
        show_members_archived: value.show_members_archived,
        show_custom_fronts: value.show_custom_fronts,
        respect_front_notifications_disabled: value.respect_front_notifications_disabled,
        privacy_fine_grained: value.privacy_fine_grained,
        privacy_fine_grained_buckets: value.privacy_fine_grained_buckets.clone(),
        enable_website: value.enable_website,
        enable_discord: value.enable_discord,
        enable_discord_status_message: value.enable_discord_status_message,
        enable_vrchat: value.enable_vrchat,
        enable_to_pluralkit: value.enable_to_pluralkit,
        simply_plural_token: value.simply_plural_token.clone(),
        discord_status_message_token: value.discord_status_message_token.clone(),
        vrchat_username: value.vrchat_username.clone(),
        vrchat_password: value.vrchat_password.clone(),
        vrchat_cookie: value.vrchat_cookie.clone(),
        pluralkit_token: value.pluralkit_token.clone(),
        history_limit: value.history_limit,
        history_truncate_after_days: value.history_truncate_after_days,
        fronter_channel_wait_increment: value.fronter_channel_wait_increment,
    }
}
