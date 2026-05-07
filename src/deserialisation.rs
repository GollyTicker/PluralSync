use anyhow::Result;
use serde::Deserialize;
use serde::Deserializer;

pub fn parse_epoch_millis_to_datetime_utc<'de, D>(
    d: D,
) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let epoch_millis = i64::deserialize(d)?;
    chrono::DateTime::from_timestamp_millis(epoch_millis)
        .ok_or_else(|| serde::de::Error::custom("Datime<Utc> from timestamp failed"))
}

pub fn deserialize_non_empty_string_as_option<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    let non_empty_str_option = if s.is_empty() { None } else { Some(s) };
    Ok(non_empty_str_option)
}

pub fn deserialize_non_empty_string<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    if s.is_empty() {
        return Err(serde::de::Error::custom("field must not be empty"));
    }
    Ok(s)
}

pub fn parse_rfc3339_as_option<'de, D>(
    d: D,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(d)?.map_or_else(
        || Ok(None),
        |ts| {
            chrono::DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map(Some)
                .map_err(serde::de::Error::custom)
        },
    )
}
