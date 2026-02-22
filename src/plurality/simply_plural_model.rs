use std::string::ToString;

use anyhow::Result;
use serde::Deserialize;
use serde::Deserializer;
use tokio_tungstenite::tungstenite;

pub const GLOBAL_PLURALSYNC_ON_SIMPLY_PLURAL_USER_ID: &str =
    "eb06960e5b7fb576923f0e909947c0ce8ca46dcbe61ee5af2681f8f59404df5d";

pub const SIMPLY_PLURAL_VRCHAT_STATUS_NAME_FIELD_NAME: &str = "VRChat Status Name";

#[derive(Deserialize, Debug, Clone)]
pub struct FrontEntry {
    pub content: FrontEntryContent,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FrontEntryContent {
    /** Can be a member ID OR a custom front ID */
    #[serde(rename = "member")]
    pub fronter_id: String,

    #[serde(rename = "uid")]
    pub system_id: String,

    #[serde(rename = "startTime")]
    #[serde(deserialize_with = "parse_epoch_millis_to_datetime_utc")]
    pub start_time: chrono::DateTime<chrono::Utc>,
}

fn parse_epoch_millis_to_datetime_utc<'de, D>(
    d: D,
) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let epoch_millis = i64::deserialize(d)?;
    chrono::DateTime::from_timestamp_millis(epoch_millis)
        .ok_or_else(|| serde::de::Error::custom("Datime<Utc> from timestamp failed"))
}

fn deserialize_non_empty_string_as_option<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    let non_empty_str_option = if s.is_empty() { None } else { Some(s) };
    Ok(non_empty_str_option)
}

#[derive(Debug, Clone)]
pub struct Fronter {
    pub fronter_id: String,
    pub name: String,
    pub avatar_url: String,
    pub vrchat_status_name: Option<String>,
    pub pluralkit_id: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub privacy_buckets: Vec<String>,
}

impl Fronter {
    #[must_use]
    pub fn preferred_vrchat_status_name(&self) -> &str {
        self.vrchat_status_name.as_ref().unwrap_or(&self.name)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CustomFront {
    pub content: CustomFrontContent,
    #[serde(rename = "id")]
    pub custom_front_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CustomFrontContent {
    pub name: String,

    #[serde(rename = "avatarUrl")]
    #[serde(default)]
    pub avatar_url: String,

    #[serde(rename = "buckets")]
    #[serde(default)]
    pub privacy_buckets: Vec<String>,
}

impl From<CustomFront> for Fronter {
    fn from(cf: CustomFront) -> Self {
        Self {
            fronter_id: cf.custom_front_id,
            name: cf.content.name,
            avatar_url: cf.content.avatar_url,
            vrchat_status_name: None,
            pluralkit_id: None,
            start_time: None,
            privacy_buckets: cf.content.privacy_buckets,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Member {
    pub content: MemberContent,
    #[serde(rename = "id")]
    pub member_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MemberContent {
    pub name: String,

    #[serde(rename = "avatarUrl")]
    #[serde(default)]
    pub avatar_url: String,

    #[serde(default)]
    pub info: serde_json::Value,
    // if the user uses the custom field "VRChat Status Name" on this member, then this will be
    // { "<vrcsn_field_id>": "<vrcsn>", ...}
    #[serde(default)]
    pub archived: bool,

    #[serde(rename = "preventsFrontNotifs")]
    #[serde(default)]
    pub front_notifications_disabled: bool,

    /* the fields `private` and `preventTrusted` are always true for all members (according to our testing)!
    so it doesn't mean what we mean by member privacy.
    hence, we don't include it in our implementation
    */
    #[serde(rename = "buckets")]
    #[serde(default)]
    pub privacy_buckets: Vec<String>,

    #[serde(rename = "pkId")]
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_non_empty_string_as_option")]
    pub pluralkit_id: Option<String>,

    // this will be populated later after deserialisation
    #[serde(default)]
    pub vrcsn_field_id: Option<String>,
}

impl From<Member> for Fronter {
    fn from(m: Member) -> Self {
        let vrchat_status_name = m.content.vrcsn_field_id.as_ref().and_then(|field_id| {
            m.content
                .info
                .as_object()
                .and_then(|custom_fields| custom_fields.get(field_id))
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        });
        Self {
            fronter_id: m.member_id,
            name: m.content.name,
            avatar_url: m.content.avatar_url,
            vrchat_status_name,
            pluralkit_id: m.content.pluralkit_id,
            start_time: None,
            privacy_buckets: m.content.privacy_buckets,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CustomField {
    pub id: String, // custom field id
    pub content: CustomFieldContent,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CustomFieldContent {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Friend {
    pub content: FriendContent,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FriendContent {
    #[serde(rename = "frienduid")]
    pub friend_user_id: String,

    #[serde(rename = "buckets")]
    #[serde(default)]
    pub assigned_privacy_buckets: Vec<String>,
}

pub fn relevantly_changed_based_on_simply_plural_websocket_event(
    message: &tungstenite::Utf8Bytes,
) -> Result<bool> {
    let event = serde_json::from_str(message).inspect_err(|e| {
        log::warn!(
            "# | relevantly_changed_based_on_simply_plural_websocket_event | {e} | input: {}",
            message.chars().take(500).collect::<String>()
        );
    })?;

    /*
    all possible collections are here:
    https://docs.apparyllis.com/docs/getting-started/collections

    collections we MUST NOT ignore:
      friends, pendingFriendRequests: when our PluralSync gets friends or changes there
      frontStatuses: even if we don't display it currently, but maybe in future we will
      frontHistory: obviously
      members: obviously
      private: just to be sure...
      tokens: just to be sure...
      users: just to be sure...

    usually we get a json with {msg: "update", target: <collection>, ...}
    However, we can also get a notification: {msg: "notification", title: <title>, message: <message>}
    How should we proceed in such a situation? For now, we'll simply take each notification as if it's a system change

    furthermore, we still want to force-reset the cache e.g. once per hour, so any things we've missed would be caught
    */

    let irrelevant_change = matches!(
        event,
        Event {
            msg: "update",
            // collections we can safely ignore
            target: Some(
                "automatedReminders"
                    | "channel"
                    | "channelCategories"
                    | "chatMessages"
                    | "groups"
                    | "notes"
                    | "polls"
                    | "repeatedReminders"
            )
        }
    );

    Ok(!irrelevant_change)
}

/** The Message as sent by Simply Plural on the Websocket.
 *
 * We use &str to make the code for parsing look better and simpler by being able to match against &str literals.
*/
#[derive(Debug, Clone, Deserialize)]
struct Event<'a> {
    msg: &'a str,
    target: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::tungstenite;

    #[test]
    fn test_relevantly_changed_based_on_simply_plural_websocket_event() {
        let utf8_bytes =
            tungstenite::Utf8Bytes::from("{\"msg\": \"update\", \"target\": \"notes\"}");
        assert!(!relevantly_changed_based_on_simply_plural_websocket_event(&utf8_bytes).unwrap());

        let utf8_bytes =
            tungstenite::Utf8Bytes::from("{\"msg\": \"update\", \"target\": \"members\"}");
        assert!(relevantly_changed_based_on_simply_plural_websocket_event(&utf8_bytes).unwrap());

        let utf8_bytes =
            tungstenite::Utf8Bytes::from("{\"msg\": \"notification\", \"title\": \"Test\"}");
        assert!(relevantly_changed_based_on_simply_plural_websocket_event(&utf8_bytes).unwrap());
    }

    #[test]
    fn test_member_json_pluralkid_id_empty_string_as_none() {
        let json_str = r#"
        {
            "id": "member1",
            "content": {
                "name": "Test Member",
                "avatarUrl": "",
                "info": {},
                "archived": false,
                "preventsFrontNotifs": false,
                "buckets": [],
                "pkId": ""
            }
        }
        "#;
        let member: Member = serde_json::from_str(json_str).unwrap();
        assert_eq!(member.content.pluralkit_id, None);
    }

    #[test]
    fn test_member_json_pluralkid_id_missing_as_none() {
        let json_str = r#"
        {
            "id": "member1",
            "content": {
                "name": "Test Member",
                "avatarUrl": "",
                "info": {},
                "archived": false,
                "preventsFrontNotifs": false,
                "buckets": []
            }
        }
        "#;
        let member: Member = serde_json::from_str(json_str).unwrap();
        assert_eq!(member.content.pluralkit_id, None);
    }
}
