pub mod clock;
pub mod communication;
pub mod controlflow;
pub mod license;
pub mod meta;
pub mod platforms;
pub mod updater;
pub mod users;

#[cfg(test)]
mod communication_tests;

pub mod for_discord_bridge {
    pub use crate::communication::{
        BridgeToServerSseMessage, FireAndForgetChannel, LatestReceiver, ServerToBridgeSseMessage,
        blocking_abort_and_clear_tasks, fire_and_forget_channel,
    };
    pub use crate::license;
    pub use crate::meta::{
        CANONICAL_PLURALSYNC_BASE_URL, PLURALSYNC_VERSION, PluralSyncVariantInfo,
    };
    pub use crate::platforms::DiscordRichPresence;
    pub use crate::updater::UpdaterStatus;
    pub use crate::users::{JwtString, UserLoginCredentials, UserProvidedPassword};
}
