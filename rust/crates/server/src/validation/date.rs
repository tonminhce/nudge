pub const NON_ISO_DATE_FORMATS: [&str; 11] = [
    "yyyy.MM.dd",
    "yyyy/MM/dd",
    "MM-dd-yyyy",
    "MM/dd/yyyy",
    "MM.dd.yyyy",
    "MMMM d, yyyy",
    "MMM d, yyyy",
    "d MMMM yyyy",
    "d MMM yyyy",
    "dd-MMM-yyyy",
    "yyyy-MMM-dd",
];

pub const ACCEPTED_DATE_FORMATS: [&str; 19] = [
    "yyyy-MM-dd",
    "yyyyMMdd",
    "yyyy.MM.dd",
    "yyyy/MM/dd",
    "MM-dd-yyyy",
    "MM/dd/yyyy",
    "MM.dd.yyyy",
    "MMMM d, yyyy",
    "MMM d, yyyy",
    "d MMMM yyyy",
    "d MMM yyyy",
    "dd-MMM-yyyy",
    "yyyy-MMM-dd",
    "yyyy-MM-dd'T'HH:mm:ss.SSSX",
    "yyyy-MM-dd'T'HH:mm:ssX",
    "yyyy-MM-dd'T'HH:mm:ss.SSS",
    "yyyy-MM-dd'T'HH:mm:ss",
    "yyyy-MM-dd HH:mm:ss",
    "yyyy-MM-dd HH:mm:ss.SSS",
];

const MONTH_LONG_NAMES: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

const MONTH_SHORT_NAMES: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

const MONTH_NARROW_NAMES: [&str; 12] =
    ["j", "f", "m", "a", "m", "j", "j", "a", "s", "o", "n", "d"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub offset_minutes: Option<i32>,
}

enum Token {
    Year,
    MonthNumeric,
    MonthShort,
    MonthLong,
    Day,
    Hour,
    Minute,
    Second,
    Fraction,
    Zone,
    Literal(char),
}

fn tokenize_format(format: &str) -> Vec<Token> {
    let chars: Vec<char> = format.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        if ch == '\'' {
            index += 1;

            while index < chars.len() {
                if chars[index] == '\'' {
                    if chars.get(index + 1) == Some(&'\'') {
                        tokens.push(Token::Literal('\''));
                        index += 2;
                        continue;
                    }

                    index += 1;
                    break;
                }

                tokens.push(Token::Literal(chars[index]));
                index += 1;
            }

            continue;
        }

        let mut run = 1;

        while index + run < chars.len() && chars[index + run] == ch {
            run += 1;
        }

        tokens.push(match (ch, run) {
            ('y', _) => Token::Year,
            ('M', 4) => Token::MonthLong,
            ('M', 3) => Token::MonthShort,
            ('M', _) => Token::MonthNumeric,
            ('d', _) => Token::Day,
            ('H', _) => Token::Hour,
            ('m', 2) => Token::Minute,
            ('m', _) => Token::Minute,
            ('s', _) => Token::Second,
            ('S', _) => Token::Fraction,
            ('X', _) => Token::Zone,
            _ => Token::Literal(ch),
        });

        index += run;
    }

    tokens
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn take_digits(value: &str, max: usize) -> Option<(i64, &str)> {
    let mut taken = 0usize;

    for ch in value.chars() {
        if taken == max || !ch.is_ascii_digit() {
            break;
        }

        taken += ch.len_utf8();
    }

    if taken == 0 {
        return None;
    }

    value[..taken]
        .parse::<i64>()
        .ok()
        .map(|number| (number, &value[taken..]))
}

fn take_exact_digits(value: &str, count: usize) -> Option<(u32, &str)> {
    let bytes = value.as_bytes();

    if bytes.len() < count || !bytes[..count].iter().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    value[..count]
        .parse::<u32>()
        .ok()
        .map(|number| (number, &value[count..]))
}

fn match_month_name<'a>(value: &'a str, names: &[&str; 12]) -> Option<(usize, &'a str)> {
    let lowered = value.to_lowercase();

    for (index, name) in names.iter().enumerate() {
        if lowered.starts_with(name) {
            return Some((index, &value[name.len()..]));
        }
    }

    None
}

fn match_zone(value: &str) -> Option<(i32, &str)> {
    if let Some(rest) = value.strip_prefix('Z') {
        return Some((0, rest));
    }

    let sign = match value.as_bytes().first() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return None,
    };

    let (hours, rest) = take_exact_digits(&value[1..], 2)?;
    let has_minutes = rest.len() >= 2
        && rest
            .as_bytes()
            .iter()
            .take(2)
            .all(|byte| byte.is_ascii_digit());
    let (minutes, rest) = if has_minutes {
        take_exact_digits(rest, 2)?
    } else {
        (0, rest)
    };

    Some((sign * ((hours as i32) * 60 + minutes as i32), rest))
}

pub fn parse_with_format(value: &str, format: &str) -> Option<ParsedDate> {
    let mut rest = value;
    let mut year = 0i32;
    let mut month = 0u32;
    let mut day = 0u32;
    let mut hour = 0u32;
    let mut minute = 0u32;
    let mut second = 0u32;
    let mut offset_minutes = None;

    for token in tokenize_format(format) {
        match token {
            Token::Year => {
                let (digits, tail) = take_digits(rest, 4)?;

                if digits < 1 || digits > 9999 {
                    return None;
                }

                year = digits as i32;
                rest = tail;
            }
            Token::MonthNumeric => {
                let (digits, tail) = take_digits(rest, 2)?;

                if digits < 1 || digits > 12 {
                    return None;
                }

                month = digits as u32;
                rest = tail;
            }
            Token::MonthShort => {
                let (index, tail) = match_month_name(rest, &MONTH_SHORT_NAMES)
                    .or_else(|| match_month_name(rest, &MONTH_NARROW_NAMES))?;

                month = index as u32 + 1;
                rest = tail;
            }
            Token::MonthLong => {
                let (index, tail) = match_month_name(rest, &MONTH_LONG_NAMES)
                    .or_else(|| match_month_name(rest, &MONTH_SHORT_NAMES))
                    .or_else(|| match_month_name(rest, &MONTH_NARROW_NAMES))?;

                month = index as u32 + 1;
                rest = tail;
            }
            Token::Day => {
                let (digits, tail) = take_digits(rest, 2)?;

                day = digits as u32;
                rest = tail;
            }
            Token::Hour => {
                let (digits, tail) = take_digits(rest, 2)?;

                if digits > 23 {
                    return None;
                }

                hour = digits as u32;
                rest = tail;
            }
            Token::Minute => {
                let (digits, tail) = take_digits(rest, 2)?;

                if digits > 59 {
                    return None;
                }

                minute = digits as u32;
                rest = tail;
            }
            Token::Second => {
                let (digits, tail) = take_digits(rest, 2)?;

                if digits > 59 {
                    return None;
                }

                second = digits as u32;
                rest = tail;
            }
            Token::Fraction => {
                let (_, tail) = take_digits(rest, 3)?;

                rest = tail;
            }
            Token::Zone => {
                let (parsed, tail) = match_zone(rest)?;

                offset_minutes = Some(parsed);
                rest = tail;
            }
            Token::Literal(ch) => {
                if rest.chars().next() != Some(ch) {
                    return None;
                }

                rest = &rest[ch.len_utf8()..];
            }
        }
    }

    if !rest.chars().all(|ch| ch.is_whitespace()) {
        return None;
    }

    if day < 1 || day > days_in_month(year, month) {
        return None;
    }

    Some(ParsedDate {
        year,
        month,
        day,
        hour,
        minute,
        second,
        offset_minutes,
    })
}

pub fn is_valid_date_format(value: &str) -> bool {
    ACCEPTED_DATE_FORMATS
        .iter()
        .any(|format| parse_with_format(value, format).is_some())
}

pub fn is_year_written_in_full(value: &str, year: i32) -> bool {
    value.contains(&format!("{year:04}"))
}

pub fn parse_non_iso_instant(value: &str) -> Option<ParsedDate> {
    NON_ISO_DATE_FORMATS.iter().find_map(|format| {
        let parsed = parse_with_format(value, format)?;

        is_year_written_in_full(value, parsed.year).then_some(parsed)
    })
}

fn split_iso_at_time(value: &str) -> Option<(&str, &str)> {
    for ch in ['T', 't', ' '] {
        if let Some(index) = value.find(ch) {
            return Some((&value[..index], &value[index + ch.len_utf8()..]));
        }
    }

    None
}

fn match_iso_date(value: &str) -> Option<(i32, u32, u32)> {
    let bytes = value.as_bytes();

    let parts = if bytes.len() == 8 && bytes.iter().all(|byte| byte.is_ascii_digit()) {
        (
            value[..4].parse::<i32>().ok()?,
            value[4..6].parse::<u32>().ok()?,
            value[6..8].parse::<u32>().ok()?,
        )
    } else if bytes.len() == 10
        && bytes[..4].iter().all(|byte| byte.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|byte| byte.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..].iter().all(|byte| byte.is_ascii_digit())
    {
        (
            value[..4].parse::<i32>().ok()?,
            value[5..7].parse::<u32>().ok()?,
            value[8..10].parse::<u32>().ok()?,
        )
    } else {
        return None;
    };

    Some(parts)
}

fn match_iso_time(value: &str) -> Option<(u32, u32, u32, &str)> {
    let (hour, rest) = take_exact_digits(value, 2)?;
    let (minute, rest) = match rest.strip_prefix(':') {
        Some(tail) => take_exact_digits(tail, 2)?,
        None => take_exact_digits(rest, 2)?,
    };

    if hour > 23 || minute > 59 {
        return None;
    }

    let (second, rest) = match rest.strip_prefix(':') {
        Some(tail) => {
            let (second, tail) = take_exact_digits(tail, 2)?;

            if second > 59 {
                return None;
            }

            (second, tail)
        }
        None => (0, rest),
    };

    let rest = match rest.strip_prefix('.') {
        Some(tail) => {
            let digits = tail
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .count();

            if digits == 0 {
                return None;
            }

            &tail[digits..]
        }
        None => rest,
    };

    Some((hour, minute, second, rest))
}

fn strip_annotation(value: &str) -> &str {
    match value.find('[') {
        Some(index) => &value[..index],
        None => value,
    }
}

pub fn parse_iso_instant(value: &str) -> Option<ParsedDate> {
    let (date_part, time_part) = split_iso_at_time(strip_annotation(value))?;
    let (year, month, day) = match_iso_date(date_part)?;
    let (hour, minute, second, rest) = match_iso_time(time_part)?;
    let (offset_minutes, rest) = match_zone(rest)?;

    if !rest.is_empty() || day < 1 || day > days_in_month(year, month) {
        return None;
    }

    Some(ParsedDate {
        year,
        month,
        day,
        hour,
        minute,
        second,
        offset_minutes: Some(offset_minutes),
    })
}

pub fn parse_iso_plain_date_time(value: &str) -> Option<ParsedDate> {
    let value = strip_annotation(value);

    if let Some((date_part, time_part)) = split_iso_at_time(value) {
        let (year, month, day) = match_iso_date(date_part)?;
        let (hour, minute, second, rest) = match_iso_time(time_part)?;

        if !rest.is_empty() || day < 1 || day > days_in_month(year, month) {
            return None;
        }

        return Some(ParsedDate {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset_minutes: None,
        });
    }

    let (year, month, day) = match_iso_date(value)?;

    if day < 1 || day > days_in_month(year, month) {
        return None;
    }

    Some(ParsedDate {
        year,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0,
        offset_minutes: None,
    })
}

pub fn parse_to_instant_or_throw(value: &str) -> Option<ParsedDate> {
    parse_iso_instant(value)
        .or_else(|| parse_iso_plain_date_time(value))
        .or_else(|| parse_non_iso_instant(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_date_formats_match_the_pinned_date_fns_behaviour() {
        for value in [
            "2024-01-01",
            "2024-1-1",
            "20240101",
            "01-02-2024",
            "1/2/2024",
            "1.2.2024",
            "2024.01.01",
            "2024/1/1",
            "January 1, 2024",
            "Jan 1, 2024",
            "1 January 2024",
            "1 Jan 2024",
            "01-Jan-2024",
            "2024-Jan-01",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00",
            "2024-01-01T00:00:00.000Z",
            "2024-01-01T00:00:00.000",
            "2024-01-01 00:00:00",
            "2024-01-01 00:00:00.000",
            "24-01-01",
            "DEC 25, 2024",
            "31.01.2024",
            "2024-02-29",
            "2024-01-01T00:00:00+0000",
            "2024-01-01T00:00:00.123+0700",
        ] {
            assert!(is_valid_date_format(value), "expected {value} to parse");
        }

        for value in [
            "",
            "garbage",
            "2024",
            "1",
            "2024-01",
            "2024-13-01",
            "2024-01-32",
            "2024-02-30",
            "2024-01-00",
            "2024-00-01",
            "999999-01-01",
            "12345678",
            "240101",
            "01/01/2024extra",
            "2024-01-01T13:45:00+07:00",
            "2024-01-01T25:00:00",
            "2024-01-01T24:00:00",
            "2024-01-01T00:60:00",
            "2024-01-01T00:00:60",
            "2024-01-01t00:00:00",
            "  2024-01-01  ",
            "Jan 1 2024",
            "January 1, 2024x",
            "2024-01-01X",
            "0000-01-01",
            "19999-01-01",
            "Janu 1, 2024",
            "Ja 1, 2024",
            "2024-01-01T000:00:00Z",
            "2024-01-01T00:00:00+7",
            "2024-01-01T00:00:00+00:00",
            "2024-01-01T00:00:00.",
        ] {
            assert!(!is_valid_date_format(value), "expected {value} to fail");
        }
    }

    #[test]
    fn narrow_and_abbreviated_month_resolution_matches_date_fns() {
        for (value, month) in [
            ("J 1, 2024", 1),
            ("F 1, 2024", 2),
            ("M 1, 2024", 3),
            ("A 1, 2024", 4),
            ("S 1, 2024", 9),
            ("O 1, 2024", 10),
            ("N 1, 2024", 11),
            ("D 1, 2024", 12),
            ("Mar 1, 2024", 3),
            ("May 1, 2024", 5),
        ] {
            assert_eq!(
                parse_with_format(value, "MMM d, yyyy").map(|parsed| parsed.month),
                Some(month),
                "unexpected month for {value}"
            );
        }

        assert_eq!(
            parse_with_format("March 1, 2024", "MMMM d, yyyy").map(|parsed| parsed.month),
            Some(3)
        );
        assert_eq!(
            parse_with_format("Jan 1, 2024", "MMMM d, yyyy").map(|parsed| parsed.month),
            Some(1)
        );
        assert_eq!(
            parse_with_format("May 1, 2024", "MMMM d, yyyy").map(|parsed| parsed.month),
            Some(5)
        );
        assert!(parse_with_format("January 1, 2024", "MMM d, yyyy").is_none());
    }

    #[test]
    fn day_is_validated_against_the_parsed_month_whatever_the_token_order() {
        assert!(parse_with_format("31-Jan-2024", "dd-MMM-yyyy").is_some());
        assert!(parse_with_format("29-Feb-2023", "dd-MMM-yyyy").is_none());
        assert!(parse_with_format("31.04.2024", "yyyy.MM.dd").is_none());
        assert!(parse_with_format("2024.04.31", "yyyy.MM.dd").is_none());
    }

    #[test]
    fn iso_instant_requires_an_offset_and_a_plain_date_does_not() {
        assert!(parse_iso_instant("2024-01-01T00:00:00Z").is_some());
        assert!(parse_iso_instant("2024-01-01T00:00Z").is_some());
        assert!(parse_iso_instant("2024-01-01 00:00:00+07:00").is_some());
        assert!(parse_iso_instant("2024-01-01T00:00:00").is_none());
        assert!(parse_iso_instant("2024-01-01").is_none());
        assert!(parse_iso_plain_date_time("2024-01-01T00:00:00").is_some());
        assert!(parse_iso_plain_date_time("2024-01-01").is_some());
        assert!(parse_iso_plain_date_time("20240101").is_some());
        assert!(parse_iso_plain_date_time("2024-1-1").is_none());
        assert!(parse_iso_plain_date_time("2024-02-30").is_none());
    }

    #[test]
    fn instant_parsing_falls_back_to_non_iso_formats() {
        assert!(parse_to_instant_or_throw("2024-01-01").is_some());
        assert!(parse_to_instant_or_throw("2024-01-01T00:00:00").is_some());
        assert!(parse_to_instant_or_throw("01-Jan-2024").is_some());
        assert!(parse_to_instant_or_throw("2024-02-30").is_none());
        assert!(parse_to_instant_or_throw("01/02/2024").is_some());
        assert!(parse_to_instant_or_throw("2024-01-01T13:45:00+07:00").is_some());
    }

    #[test]
    fn year_written_in_full_blocks_two_digit_years() {
        assert!(is_year_written_in_full("01-Jan-2024", 2024));
        assert!(!is_year_written_in_full("01-Jan-24", 24));
        assert!(is_year_written_in_full("01-Jan-0024", 24));
        assert!(parse_non_iso_instant("01-Jan-24").is_none());
        assert!(parse_non_iso_instant("01-Jan-0024").is_some());
    }
}
