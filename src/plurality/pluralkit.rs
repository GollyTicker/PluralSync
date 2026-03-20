use serde::{Deserialize, Serialize};

/// See: <https://pluralkit.me/api/dispatch>/
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluralKitWebhookEvent {
    Ping,
    UpdateSystem,
    UpdateSettings,
    CreateMember,
    UpdateMember,
    DeleteMember,
    CreateGroup,
    UpdateGroup,
    UpdateGroupMembers,
    DeleteGroup,
    LinkAccount,
    UnlinkAccount,
    UpdateSystemGuild,
    UpdateMemberGuild,
    CreateMessage,
    CreateSwitch,
    UpdateSwitch,
    DeleteSwitch,
    DeleteAllSwitches,
    SuccessfulImport,
    UpdateAutoproxy,
}

impl PluralKitWebhookEvent {
    #[must_use]
    pub const fn can_be_ignored_for_purppose_of_syncing(&self) -> bool {
        matches!(
            self,
            Self::Ping
                | Self::UpdateSettings
                | Self::CreateGroup
                | Self::UpdateGroup
                | Self::UpdateGroupMembers
                | Self::DeleteGroup
                | Self::LinkAccount
                | Self::UnlinkAccount
                | Self::UpdateSystemGuild
                | Self::UpdateMemberGuild
                | Self::UpdateAutoproxy
        )
    }

    /// Returns true if this is a ping event (for health monitoring).
    #[must_use]
    pub const fn is_ping(&self) -> bool {
        matches!(self, Self::Ping)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluralKitWebhookPayload {
    /// The event type
    #[serde(rename = "type")]
    pub event_type: PluralKitWebhookEvent,
    pub signing_token: String,
    pub system_id: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_is_ping() {
        assert!(PluralKitWebhookEvent::Ping.is_ping());
        assert!(!PluralKitWebhookEvent::CreateSwitch.is_ping());
    }

    #[test]
    fn test_payload_deserialize_switch_event() {
        let json = r#"{
            "type": "CREATE_SWITCH",
            "signing_token": "test-secret-token",
            "system_id": "sys_abc123",
            "data": {
                "members": ["mem_1", "mem_2"],
                "timestamp": "2024-01-01T00:00:00Z"
            }
        }"#;

        let payload: PluralKitWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(
            payload.event_type,
            PluralKitWebhookEvent::CreateSwitch
        ));
        assert_eq!(payload.system_id, "sys_abc123");
        assert_eq!(payload.signing_token, "test-secret-token");
        assert!(payload.data.is_some());
    }

    #[test]
    fn test_payload_deserialize_ping() {
        let json = r#"{
            "type": "PING",
            "signing_token": "test-secret-token",
            "system_id": "sys_abc123"
        }"#;

        let payload: PluralKitWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(payload.event_type, PluralKitWebhookEvent::Ping));
        assert_eq!(payload.system_id, "sys_abc123");
        assert_eq!(payload.signing_token, "test-secret-token");
        assert!(payload.data.is_none());
        assert!(payload.id.is_none());
    }

    #[test]
    fn test_payload_deserialize_member_event_with_id() {
        let json = r#"{
            "type": "UPDATE_MEMBER",
            "signing_token": "test-secret-token",
            "system_id": "sys_abc123",
            "id": "mem_def456",
            "data": {
                "name": "New Member Name",
                "color": "FF5733"
            }
        }"#;

        let payload: PluralKitWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(matches!(
            payload.event_type,
            PluralKitWebhookEvent::UpdateMember
        ));
        assert_eq!(payload.id, Some("mem_def456".to_string()));
        assert!(payload.data.is_some());
    }
}
