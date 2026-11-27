use std::sync::OnceLock;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Duration;
use chrono::FixedOffset;
use chrono::LocalResult;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::Offset;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Utc;
use chrono_tz::Tz;
use chrono_tz::TZ_VARIANTS;
use regex::Regex;
use serde_json::json;
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    This,
    Past,
    Next,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FirstDayOfWeek {
    Monday,
    Saturday,
    Sunday,
}

#[derive(Clone)]
enum Zone {
    Named(Tz),
    Fixed(FixedOffset),
}

#[derive(Clone)]
struct ZonedDateTime {
    utc: DateTime<Utc>,
    zone: Zone,
}

impl ZonedDateTime {
    fn new(utc: DateTime<Utc>, zone: Zone) -> Self {
        Self { utc, zone }
    }

    fn local_naive(&self) -> NaiveDateTime {
        match &self.zone {
            Zone::Named(zone) => self.utc.with_timezone(zone).naive_local(),
            Zone::Fixed(zone) => self.utc.with_timezone(zone).naive_local(),
        }
    }

    fn from_local(naive: NaiveDateTime, zone: Zone) -> Result<Self, String> {
        let utc = match &zone {
            Zone::Named(zone) => match zone.from_local_datetime(&naive) {
                LocalResult::Single(value) => value.with_timezone(&Utc),
                LocalResult::Ambiguous(first, second) => first.min(second).with_timezone(&Utc),
                LocalResult::None => {
                    let before_seconds = (naive - Duration::hours(26))
                        .and_utc()
                        .with_timezone(zone)
                        .offset()
                        .fix()
                        .local_minus_utc();
                    (naive - Duration::seconds(before_seconds as i64)).and_utc()
                }
            },
            Zone::Fixed(zone) => naive
                .and_local_timezone(*zone)
                .single()
                .ok_or_else(|| String::from("Cannot resolve relative date timezone"))?
                .with_timezone(&Utc),
        };
        Ok(Self { utc, zone })
    }

    fn add_calendar(self, unit: Unit, amount: i64) -> Result<Self, String> {
        let local = self.local_naive();
        let updated = match unit {
            Unit::Day => local
                .checked_add_signed(
                    Duration::try_days(amount)
                        .ok_or_else(|| String::from("Relative date is out of range"))?,
                )
                .ok_or_else(|| String::from("Relative date is out of range"))?,
            Unit::Week => local
                .checked_add_signed(
                    Duration::try_days(amount.saturating_mul(7))
                        .ok_or_else(|| String::from("Relative date is out of range"))?,
                )
                .ok_or_else(|| String::from("Relative date is out of range"))?,
            Unit::Month => add_months(local, amount)?,
            Unit::Quarter => add_months(local, amount.saturating_mul(3))?,
            Unit::Year => add_months(local, amount.saturating_mul(12))?,
            _ => return Err(String::from("Relative date unit is not a calendar unit")),
        };
        Self::from_local(updated, self.zone)
    }

    fn add_elapsed(self, unit: Unit, amount: i64) -> Result<Self, String> {
        let duration = match unit {
            Unit::Second => Duration::try_seconds(amount),
            Unit::Minute => Duration::try_minutes(amount),
            Unit::Hour => Duration::try_hours(amount),
            _ => return Err(String::from("Relative date unit is not an elapsed unit")),
        }
        .ok_or_else(|| String::from("Relative date is out of range"))?;
        let utc = self
            .utc
            .checked_add_signed(duration)
            .ok_or_else(|| String::from("Relative date is out of range"))?;
        Ok(Self {
            utc,
            zone: self.zone,
        })
    }

    fn add(self, unit: Unit, amount: i64) -> Result<Self, String> {
        match unit {
            Unit::Second | Unit::Minute | Unit::Hour => self.add_elapsed(unit, amount),
            _ => self.add_calendar(unit, amount),
        }
    }

    fn subtract(self, unit: Unit, amount: i64) -> Result<Self, String> {
        self.add(unit, -amount)
    }

    fn period_start(self, unit: Unit, first_day_of_week: FirstDayOfWeek) -> Result<Self, String> {
        let local = self.local_naive();
        let date = local.date();
        let updated = match unit {
            Unit::Day => date.and_hms_opt(0, 0, 0),
            Unit::Week => {
                let first = match first_day_of_week {
                    FirstDayOfWeek::Monday => 1,
                    FirstDayOfWeek::Saturday => 6,
                    FirstDayOfWeek::Sunday => 7,
                };
                let offset = (date.weekday().number_from_monday() as i64 - first + 7) % 7;
                date.checked_sub_signed(Duration::days(offset))
                    .and_then(|date| date.and_hms_opt(0, 0, 0))
            }
            Unit::Month => NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
                .and_then(|date| date.and_hms_opt(0, 0, 0)),
            Unit::Quarter => {
                let month = ((date.month() - 1) / 3) * 3 + 1;
                NaiveDate::from_ymd_opt(date.year(), month, 1)
                    .and_then(|date| date.and_hms_opt(0, 0, 0))
            }
            Unit::Year => NaiveDate::from_ymd_opt(date.year(), 1, 1)
                .and_then(|date| date.and_hms_opt(0, 0, 0)),
            Unit::Hour => local
                .with_nanosecond(0)
                .and_then(|value| value.with_second(0).and_then(|value| value.with_minute(0))),
            Unit::Minute => local
                .with_nanosecond(0)
                .and_then(|value| value.with_second(0)),
            Unit::Second => local.with_nanosecond(0),
        }
        .ok_or_else(|| String::from("Cannot resolve relative period start"))?;
        Self::from_local(updated, self.zone.clone())
    }

    fn next_period_start(
        self,
        unit: Unit,
        first_day_of_week: FirstDayOfWeek,
    ) -> Result<Self, String> {
        self.period_start(unit, first_day_of_week)?.add(unit, 1)
    }
}

fn add_months(value: NaiveDateTime, months: i64) -> Result<NaiveDateTime, String> {
    let month_index = (value.year() as i64).saturating_mul(12) + value.month0() as i64 + months;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    if !(i32::MIN as i64..=i32::MAX as i64).contains(&year) {
        return Err(String::from("Relative date is out of range"));
    }
    let day = value.day().min(days_in_month(year as i32, month));
    NaiveDate::from_ymd_opt(year as i32, month, day)
        .and_then(|date| date.and_hms_opt(value.hour(), value.minute(), value.second()))
        .and_then(|date| date.with_nanosecond(value.nanosecond()))
        .ok_or_else(|| String::from("Relative date is out of range"))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn regex() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"((?:THIS)|(?:PAST)|(?:NEXT))_(\d*)_(DAY|MONTH|YEAR|WEEK|QUARTER|HOUR|MINUTE|SECOND)(?:(?:;;([^;;]*);;)?(?:(MONDAY|SUNDAY|SATURDAY);;)?)?")
            .expect("relative date regex")
    })
}

fn parse_direction(raw: &str) -> Result<Direction, String> {
    match raw {
        "THIS" => Ok(Direction::This),
        "PAST" => Ok(Direction::Past),
        "NEXT" => Ok(Direction::Next),
        _ => Err(String::from("Invalid relative date direction")),
    }
}

fn parse_unit(raw: &str) -> Result<Unit, String> {
    match raw {
        "SECOND" => Ok(Unit::Second),
        "MINUTE" => Ok(Unit::Minute),
        "HOUR" => Ok(Unit::Hour),
        "DAY" => Ok(Unit::Day),
        "WEEK" => Ok(Unit::Week),
        "MONTH" => Ok(Unit::Month),
        "QUARTER" => Ok(Unit::Quarter),
        "YEAR" => Ok(Unit::Year),
        _ => Err(String::from("Invalid relative date unit")),
    }
}

fn parse_first_day_of_week(raw: Option<&str>) -> Result<FirstDayOfWeek, String> {
    match raw {
        None => Ok(FirstDayOfWeek::Monday),
        Some("MONDAY") => Ok(FirstDayOfWeek::Monday),
        Some("SATURDAY") => Ok(FirstDayOfWeek::Saturday),
        Some("SUNDAY") => Ok(FirstDayOfWeek::Sunday),
        _ => Err(String::from("Invalid relative date first day of week")),
    }
}

fn parse_zone(raw: Option<&str>) -> Result<Zone, String> {
    let raw = match raw {
        Some(raw) => Some(raw.to_string()),
        None => std::env::var("TZ").ok().filter(|raw| !raw.is_empty()),
    };
    let Some(raw) = raw else {
        return Ok(Zone::Named(Tz::UTC));
    };
    if raw.is_empty() {
        return Err(String::from("Relative date timezone cannot be empty"));
    }
    if let Some(zone) = TZ_VARIANTS
        .iter()
        .copied()
        .find(|zone| zone.name().eq_ignore_ascii_case(&raw))
    {
        return Ok(Zone::Named(zone));
    }
    if let Some(zone) = parse_fixed_offset(&raw)? {
        return Ok(Zone::Fixed(zone));
    }
    Err(String::from("Invalid relative date timezone"))
}

fn parse_fixed_offset(raw: &str) -> Result<Option<FixedOffset>, String> {
    let pattern = Regex::new(r"^([+-])(\d{2})(?::(\d{2}))?(?::(\d{2}))?$")
        .map_err(|error| error.to_string())?;
    let Some(captures) = pattern.captures(raw) else {
        return Ok(None);
    };
    let sign = if captures.get(1).map(|value| value.as_str()) == Some("-") {
        -1
    } else {
        1
    };
    let hours = captures
        .get(2)
        .and_then(|value| value.as_str().parse::<i32>().ok())
        .ok_or_else(|| String::from("Invalid relative date timezone offset"))?;
    let minutes = captures
        .get(3)
        .map(|value| value.as_str().parse::<i32>())
        .transpose()
        .map_err(|_| String::from("Invalid relative date timezone offset"))?
        .unwrap_or(0);
    let seconds = captures
        .get(4)
        .map(|value| value.as_str().parse::<i32>())
        .transpose()
        .map_err(|_| String::from("Invalid relative date timezone offset"))?
        .unwrap_or(0);
    if hours > 23 || minutes > 59 || seconds > 59 {
        return Err(String::from("Invalid relative date timezone offset"));
    }
    FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60 + seconds))
        .map(Some)
        .ok_or_else(|| String::from("Invalid relative date timezone offset"))
}

fn format_instant(value: &ZonedDateTime) -> String {
    value.utc.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn round_to_second(value: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let increment = i64::from(value.timestamp_subsec_nanos() >= 500_000_000);
    DateTime::from_timestamp(
        value
            .timestamp()
            .checked_add(increment)
            .ok_or_else(|| String::from("Relative date is out of range"))?,
        0,
    )
    .ok_or_else(|| String::from("Relative date is out of range"))
}

pub fn resolve_filter(value: &Value, date_time: bool, now: DateTime<Utc>) -> Result<Value, String> {
    let raw = value
        .as_str()
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| String::from("Relative date filter requires a string value"))?;
    let captures = regex()
        .captures(raw)
        .ok_or_else(|| String::from("Cannot parse relative date filter"))?;
    let direction = parse_direction(
        captures
            .get(1)
            .ok_or_else(|| String::from("Relative date direction is missing"))?
            .as_str(),
    )?;
    let amount_raw = captures
        .get(2)
        .ok_or_else(|| String::from("Relative date amount is missing"))?
        .as_str();
    if amount_raw.is_empty() {
        return Err(String::from("Relative date amount must be positive"));
    }
    let amount = Some(
        amount_raw
            .parse::<i64>()
            .map_err(|_| String::from("Invalid relative date amount"))?,
    );
    if amount.is_some_and(|amount| amount <= 0) {
        return Err(String::from("Relative date amount must be positive"));
    }
    let unit = parse_unit(
        captures
            .get(3)
            .ok_or_else(|| String::from("Relative date unit is missing"))?
            .as_str(),
    )?;
    let zone = parse_zone(captures.get(4).map(|value| value.as_str()))?;
    let first_day_of_week = parse_first_day_of_week(captures.get(5).map(|value| value.as_str()))?;
    let now = if date_time {
        round_to_second(now)?
    } else {
        now
    };
    let current = ZonedDateTime::new(now, zone);
    let (start, end) = match direction {
        Direction::This => (
            current.clone().period_start(unit, first_day_of_week)?,
            current.next_period_start(unit, first_day_of_week)?,
        ),
        Direction::Past => {
            let end = current.period_start(unit, first_day_of_week)?;
            let start = end.clone().subtract(
                unit,
                amount.ok_or_else(|| String::from("Relative date amount is required"))?,
            )?;
            (start, end)
        }
        Direction::Next => {
            let start = current.next_period_start(unit, first_day_of_week)?;
            let end = start.clone().add(
                unit,
                amount.ok_or_else(|| String::from("Relative date amount is required"))?,
            )?;
            (start, end)
        }
    };
    if date_time {
        Ok(json!({
            "and": [
                { "gte": format_instant(&start) },
                { "lt": format_instant(&end) }
            ]
        }))
    } else {
        Ok(json!({
            "and": [
                { "gte": start.local_naive().date().to_string() },
                { "lt": end.local_naive().date().to_string() }
            ]
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2024-03-15T12:34:56Z")
            .expect("valid instant")
            .with_timezone(&Utc)
    }

    fn resolve(raw: &str, date_time: bool) -> Value {
        resolve_filter(&Value::String(raw.to_string()), date_time, now()).expect("relative date")
    }

    #[test]
    fn calendar_directions_match_upstream_period_boundaries() {
        assert_eq!(
            resolve("THIS_1_DAY", true),
            json!({"and": [{"gte": "2024-03-15T00:00:00Z"}, {"lt": "2024-03-16T00:00:00Z"}]})
        );
        assert_eq!(
            resolve("PAST_2_DAY", true),
            json!({"and": [{"gte": "2024-03-13T00:00:00Z"}, {"lt": "2024-03-15T00:00:00Z"}]})
        );
        assert_eq!(
            resolve("NEXT_2_MONTH", true),
            json!({"and": [{"gte": "2024-04-01T00:00:00Z"}, {"lt": "2024-06-01T00:00:00Z"}]})
        );
        assert_eq!(
            resolve("PAST_1_QUARTER", true),
            json!({"and": [{"gte": "2023-10-01T00:00:00Z"}, {"lt": "2024-01-01T00:00:00Z"}]})
        );
        assert_eq!(
            resolve("THIS_1_YEAR", true),
            json!({"and": [{"gte": "2024-01-01T00:00:00Z"}, {"lt": "2025-01-01T00:00:00Z"}]})
        );
    }

    #[test]
    fn parser_keeps_upstream_prefix_suffix_and_optional_amount_behavior() {
        assert_eq!(
            resolve("prefix THIS_9_DAY suffix", false),
            json!({"and": [{"gte": "2024-03-15"}, {"lt": "2024-03-16"}]})
        );
        assert!(resolve_filter(&Value::String(String::from("PAST_DAY")), true, now()).is_err());
        assert!(resolve_filter(&Value::String(String::from("THIS_DAY")), true, now()).is_err());
        assert!(resolve_filter(&Value::String(String::from("NEXT_0_DAY")), true, now()).is_err());
    }

    #[test]
    fn week_start_and_timezone_are_applied_before_utc_conversion() {
        assert_eq!(
            resolve("THIS_1_WEEK;;UTC;;SATURDAY;;", true),
            json!({"and": [{"gte": "2024-03-09T00:00:00Z"}, {"lt": "2024-03-16T00:00:00Z"}]})
        );
        assert_eq!(
            resolve("THIS_1_WEEK;;UTC;;SUNDAY;;", true),
            json!({"and": [{"gte": "2024-03-10T00:00:00Z"}, {"lt": "2024-03-17T00:00:00Z"}]})
        );
        assert_eq!(
            resolve("THIS_1_DAY;;+05:30;;", true),
            json!({"and": [{"gte": "2024-03-14T18:30:00Z"}, {"lt": "2024-03-15T18:30:00Z"}]})
        );
        assert_eq!(
            resolve("THIS_1_DAY;;+05;;", true),
            json!({"and": [{"gte": "2024-03-14T19:00:00Z"}, {"lt": "2024-03-15T19:00:00Z"}]})
        );
        assert_eq!(
            resolve("THIS_1_DAY;;america/new_york;;", true),
            json!({"and": [{"gte": "2024-03-15T04:00:00Z"}, {"lt": "2024-03-16T04:00:00Z"}]})
        );
    }

    #[test]
    fn daylight_saving_preserves_local_midnight_boundaries() {
        let instant = DateTime::parse_from_rfc3339("2024-03-10T12:00:00Z")
            .expect("valid instant")
            .with_timezone(&Utc);
        let value = resolve_filter(
            &Value::String(String::from("THIS_1_DAY;;America/New_York;;")),
            true,
            instant,
        )
        .expect("daylight saving date");
        assert_eq!(
            value,
            json!({"and": [{"gte": "2024-03-10T05:00:00Z"}, {"lt": "2024-03-11T04:00:00Z"}]})
        );
    }

    #[test]
    fn date_time_rounds_half_up_to_seconds_before_period_selection() {
        let instant = DateTime::parse_from_rfc3339("2024-03-15T23:59:59.600Z")
            .expect("valid instant")
            .with_timezone(&Utc);
        let second = resolve_filter(
            &Value::String(String::from("THIS_1_SECOND;;UTC;;")),
            true,
            instant,
        )
        .expect("rounded second");
        assert_eq!(
            second,
            json!({"and": [{"gte": "2024-03-16T00:00:00Z"}, {"lt": "2024-03-16T00:00:01Z"}]})
        );
        let day = resolve_filter(
            &Value::String(String::from("THIS_1_DAY;;UTC;;")),
            true,
            instant,
        )
        .expect("rounded day");
        assert_eq!(
            day,
            json!({"and": [{"gte": "2024-03-16T00:00:00Z"}, {"lt": "2024-03-17T00:00:00Z"}]})
        );
    }

    #[test]
    fn local_time_resolution_matches_temporal_compatible_disambiguation() {
        let zone = Zone::Named(Tz::from_str("America/New_York").expect("timezone"));
        let gap = NaiveDate::from_ymd_opt(2024, 3, 10)
            .expect("date")
            .and_hms_opt(2, 30, 0)
            .expect("time");
        assert_eq!(
            ZonedDateTime::from_local(gap, zone.clone())
                .expect("gap")
                .utc
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "2024-03-10T07:30:00Z"
        );
        let ambiguous = NaiveDate::from_ymd_opt(2024, 11, 3)
            .expect("date")
            .and_hms_opt(1, 30, 0)
            .expect("time");
        assert_eq!(
            ZonedDateTime::from_local(ambiguous, zone)
                .expect("ambiguous")
                .utc
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "2024-11-03T05:30:00Z"
        );
    }

    #[test]
    fn sub_day_units_use_elapsed_time_and_period_floors() {
        assert_eq!(
            resolve("NEXT_1_HOUR", true),
            json!({"and": [{"gte": "2024-03-15T13:00:00Z"}, {"lt": "2024-03-15T14:00:00Z"}]})
        );
        assert_eq!(
            resolve("PAST_1_MINUTE", true),
            json!({"and": [{"gte": "2024-03-15T12:33:00Z"}, {"lt": "2024-03-15T12:34:00Z"}]})
        );
    }

    #[test]
    fn invalid_values_and_timezones_fail_closed() {
        for value in [Value::Null, Value::String(String::new()), json!(42)] {
            assert!(resolve_filter(&value, true, now()).is_err());
        }
        for raw in [
            "THIS_1_DAY;;Not/AZone;;",
            "THIS_1_DAY;;;;",
            "THIS_1_DAY;;+05:30junk;;",
            "NEXT_9223372036854775807_DAY",
            "NEXT_9223372036854775807_HOUR",
        ] {
            assert!(resolve_filter(&Value::String(raw.to_string()), true, now()).is_err());
        }
    }
}
