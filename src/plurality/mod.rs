mod fronting_status;

#[cfg(test)]
mod fronting_status_tests;

mod model;
mod pluralkit;
mod pluralkit_webhook_api;
mod pluralkit_webhook_verification;
mod simply_plural;
mod simply_plural_model;
mod simply_plural_websocket;

pub use fronting_status::*;
pub use fronting_status::{
    CleanForPlatform, DISCORD_STATUS_MAX_LENGTH, FRONTING_STATUS_STRING, FrontingFormat,
    VRCHAT_MAX_ALLOWED_STATUS_LENGTH, format_fronting_status,
};
pub use model::*;
pub use pluralkit::*;
pub use pluralkit_webhook_api::*;
pub use pluralkit_webhook_verification::*;
pub use simply_plural::*;
pub use simply_plural_model::*;
pub use simply_plural_websocket::*;
