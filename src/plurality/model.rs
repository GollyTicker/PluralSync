use serde::{Deserialize, Serialize};

/// Generic representation of a fronter from any system source (`SimplyPlural`, `PluralKit`, etc.)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Fronter {
    pub fronter_id: String,
    pub name: String,
    pub pronouns: Option<String>,
    pub avatar_url: String,
    pub pluralkit_id: Option<String>,
    #[specta(type = String)]
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub privacy_buckets: Vec<String>,
}

/// Reasons why a fronter might be excluded from display
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ExclusionReason {
    FrontNotificationsDisabled,
    ArchivedMemberHidden,
    NonArchivedMemberHidden,
    CustomFrontsDisabled,
    NotInDisplayedPrivacyBuckets,
    MemberPrivacyPrivate,
}

/// A fronter that has been filtered, either included or excluded with a reason
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum FilteredFronter {
    Included(Fronter),
    Excluded(Fronter, ExclusionReason),
}

/// A fronter that has been excluded along with the reason
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ExcludedFronter {
    pub fronter: Fronter,
    pub reason: ExclusionReason,
}

/// Collection of filtered fronters, separated into included and excluded
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FilteredFronters {
    pub fronters: Vec<Fronter>,
    pub excluded: Vec<ExcludedFronter>,
}

impl FilteredFronter {
    #[must_use]
    pub fn into_included(self) -> Option<Fronter> {
        match self {
            Self::Included(f) => Some(f),
            Self::Excluded(_, _) => None,
        }
    }
}
