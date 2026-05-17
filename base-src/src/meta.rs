use serde::{Deserialize, Serialize};

pub const CANONICAL_PLURALSYNC_BASE_URL: &str = "https://public-test.pluralsync.ayake.net";

pub const CANONICAL_PLURALSYNC_ABOUT: &str = "https://public-test.pluralsync.ayake.net/about";

pub const PLURALSYNC_VERSION: &str = env!("PLURALSYNC_VERSION");

pub const PLURALSYNC_RELEASES_URL: &str = "https://content.radicle.ayake.net/PluralSync/releases";

pub const SIMPLY_PLURAL_DEPRECATION_DATE: chrono::DateTime<chrono::Utc> =
    chrono::NaiveDate::from_ymd_opt(2026, 6, 29)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

#[must_use]
pub fn is_simply_plural_deprecated(now: chrono::DateTime<chrono::Utc>) -> bool {
    now >= SIMPLY_PLURAL_DEPRECATION_DATE
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct PluralSyncVariantInfo {
    pub version: String,
    pub variant: String,
    pub description: Option<String>,
    pub show_in_ui: bool,
}
