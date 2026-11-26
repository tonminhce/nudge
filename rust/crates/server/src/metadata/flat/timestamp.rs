#![allow(dead_code)]

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serializer;

pub fn serialize<S: Serializer>(
    timestamp: &DateTime<Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<DateTime<Utc>, D::Error> {
    let raw = String::deserialize(deserializer)?;

    DateTime::parse_from_rfc3339(&raw)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

pub fn serialize_option<S: Serializer>(
    timestamp: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match timestamp {
        Some(timestamp) => serialize(timestamp, serializer),
        None => serializer.serialize_none(),
    }
}

pub fn deserialize_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error> {
    let raw: Option<String> = Option::deserialize(deserializer)?;

    match raw {
        Some(raw) => DateTime::parse_from_rfc3339(&raw)
            .map(|parsed| Some(parsed.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

pub fn now_to_iso_string(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
