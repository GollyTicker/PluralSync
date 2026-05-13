use serde::{Deserialize, Serialize};

pub const CANONICAL_PLURALSYNC_BASE_URL: &str = "https://public-test.pluralsync.ayake.net";

pub const CANONICAL_PLURALSYNC_ABOUT: &str = "https://public-test.pluralsync.ayake.net/about";

pub const PLURALSYNC_VERSION: &str = env!("PLURALSYNC_VERSION");

pub const PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL: &str =
    "https://github.com/GollyTicker/PluralSync/releases";

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct PluralSyncVariantInfo {
    pub version: String,
    pub variant: String,
    pub description: Option<String>,
    pub show_in_ui: bool,
}
