use serde_json::Value;

use crate::validation::error::ValidationError;
use crate::validation::error::ValidationResult;
use crate::validation::inspect::inspect;
use crate::validation::net::is_safe_url;
use crate::validation::net::is_valid_domain;
use crate::validation::net::is_valid_email;
use crate::validation::net::is_valid_uuid_v4;
use crate::validation::scalars::validate_numeric_field;
use crate::validation::scalars::validate_rating_and_select_field;
use crate::validation::scalars::validate_raw_json_field;
use crate::validation::scalars::validate_text_field;
use crate::validation::scalars::validate_uuid_field;
use crate::validation::values::is_non_empty_string;
use crate::validation::values::is_null;
use crate::validation::values::is_plain_object;
use crate::validation::values::is_string;
use crate::validation::values::object_entries;

pub const CURRENCY_CODES: [&str; 154] = [
    "AED", "AFN", "ALL", "AMD", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT", "BGN",
    "BHD", "BIF", "BMD", "BND", "BOB", "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CAD", "CDF",
    "CHF", "CLP", "CLF", "CNY", "COP", "CRC", "CUP", "CVE", "CZK", "DJF", "DKK", "DOP", "DZD",
    "EGP", "ERN", "ETB", "EUR", "FJD", "FKP", "GBP", "GEL", "GHS", "GIP", "GMD", "GNF", "GTQ",
    "GYD", "HKD", "HNL", "HTG", "HUF", "IDR", "ILS", "INR", "IQD", "IRR", "ISK", "JMD", "JOD",
    "JPY", "KES", "KGS", "KHR", "KMF", "KPW", "KRW", "KWD", "KYD", "KZT", "LAK", "LBP", "LKR",
    "LRD", "LSL", "LYD", "MAD", "MDL", "MGA", "MKD", "MMK", "MNT", "MOP", "MRU", "MUR", "MVR",
    "MWK", "MXN", "MYR", "MZN", "NAD", "NGN", "NIO", "NOK", "NPR", "NZD", "OMR", "PAB", "PEN",
    "PGK", "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD", "RUB", "RWF", "SAR", "SBD", "SCR",
    "SDG", "SEK", "SGD", "SHP", "SLE", "SOS", "SRD", "SSP", "STN", "SVC", "SYP", "SZL", "THB",
    "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TWD", "TZS", "UAH", "UGX", "USD", "UYU", "UZS",
    "VES", "VND", "VUV", "WST", "XCD", "XCG", "XOF", "YER", "ZAR", "ZMW", "ZWG",
];

pub const ACTOR_SOURCES: [&str; 10] = [
    "EMAIL",
    "CALENDAR",
    "WORKFLOW",
    "AGENT",
    "API",
    "IMPORT",
    "MANUAL",
    "SYSTEM",
    "WEBHOOK",
    "APPLICATION",
];

const TEXT_ALIGNMENTS: [&str; 4] = ["left", "center", "right", "justify"];

const COLORS: [&str; 9] = [
    "gray", "brown", "red", "orange", "yellow", "green", "blue", "purple", "pink",
];

const BLOCK_NODE_TYPES: [&str; 12] = [
    "paragraph",
    "heading",
    "bulletList",
    "orderedList",
    "taskList",
    "blockquote",
    "toggle",
    "codeBlock",
    "divider",
    "image",
    "file",
    "table",
];

const INLINE_NODE_TYPES: [&str; 3] = ["text", "hardBreak", "mentionTag"];

const MARK_TYPES: [&str; 7] = [
    "bold",
    "italic",
    "underline",
    "strike",
    "code",
    "link",
    "textStyle",
];

const MAX_HEADING_LEVEL: u64 = 6;

const MAX_INDENT_LEVEL: u64 = 8;

const URL_MAX_LENGTH: usize = 4000;

fn path(field_name: &str, sub_field: &str) -> String {
    format!("{field_name}.{sub_field}")
}

fn array_message(value: &Value, field_name: &str) -> ValidationError {
    let inspected = inspect(value);

    ValidationError::new(
        format!(
            "Invalid value {inspected} for field \"{field_name} - Array values need to be string\""
        ),
        format!("Invalid value: \"{inspected}\""),
    )
}

fn invalid_sub_field(field_kind: &str, sub_field: &str, field_name: &str) -> ValidationError {
    ValidationError::new(
        format!("Invalid subfield {sub_field} for {field_kind} field \"{field_name}\""),
        format!("Invalid value for {field_kind}."),
    )
}

fn validate_emails_primary_email_subfield(value: &Value, field_name: &str) -> ValidationResult<()> {
    if is_null(value) {
        return Ok(());
    }

    let invalid = match value {
        Value::String(text) => is_non_empty_string(value) && !is_valid_email(text),
        _ => true,
    };

    if invalid {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!("Invalid string value {inspected} for email field \"{field_name}\""),
            format!("Invalid value: \"{inspected}\""),
        ));
    }

    Ok(())
}

fn validate_emails_additional_emails_subfield(
    value: &Value,
    field_name: &str,
) -> ValidationResult<()> {
    if is_null(value) {
        return Ok(());
    }

    if value.is_string() {
        return validate_emails_primary_email_subfield(value, field_name);
    }

    let Some(items) = value.as_array() else {
        return Err(array_message(value, field_name));
    };

    for item in items {
        if item.is_null() {
            return Err(array_message(value, field_name));
        }

        validate_emails_primary_email_subfield(item, field_name)?;
    }

    Ok(())
}

pub fn validate_emails_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;

    for (sub_field, sub_value) in object_entries(value) {
        match sub_field.as_str() {
            "primaryEmail" => {
                validate_emails_primary_email_subfield(&sub_value, &path(field_name, &sub_field))?
            }
            "additionalEmails" => validate_emails_additional_emails_subfield(
                &sub_value,
                &path(field_name, &sub_field),
            )?,
            _ => return Err(invalid_sub_field("emails", &sub_field, field_name)),
        }
    }

    Ok(())
}

pub fn validate_phones_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;

    for (sub_field, sub_value) in object_entries(value) {
        let target = path(field_name, &sub_field);

        match sub_field.as_str() {
            "primaryPhoneNumber" | "primaryPhoneCountryCode" | "primaryPhoneCallingCode" => {
                validate_text_field(&sub_value, &target)?
            }
            "additionalPhones" => validate_raw_json_field(&sub_value, &target)?,
            _ => return Err(invalid_sub_field("phones", &sub_field, field_name)),
        }
    }

    Ok(())
}

fn assert_domain(value: &Value, field_name: &str) -> ValidationResult<()> {
    if !is_non_empty_string(value) {
        return Ok(());
    }

    let text = value.as_str().unwrap_or_default();

    if is_valid_domain(text) {
        return Ok(());
    }

    Err(ValidationError::new(
        format!("\"{text}\" is not a domain name, for domain-typed links field \"{field_name}\""),
        String::from("Please enter a domain name, like twenty.com"),
    ))
}

fn parse_array_or_json_string_to_array(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items.clone(),
        Value::String(text) => serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|parsed| parsed.as_array().cloned())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub fn validate_links_field(
    value: &Value,
    field_name: &str,
    links_variant: Option<&str>,
) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;

    let is_domain_field = links_variant == Some("domain");

    for (sub_field, sub_value) in object_entries(value) {
        let target = path(field_name, &sub_field);

        match sub_field.as_str() {
            "primaryLinkUrl" => {
                validate_text_field(&sub_value, &target)?;

                if is_domain_field {
                    assert_domain(&sub_value, &target)?;
                }
            }
            "primaryLinkLabel" => validate_text_field(&sub_value, &target)?,
            "secondaryLinks" => {
                validate_raw_json_field(&sub_value, &target)?;

                if is_domain_field {
                    for link in parse_array_or_json_string_to_array(&sub_value) {
                        let url = link.get("url").cloned().unwrap_or(Value::Null);

                        assert_domain(&url, &target)?;
                    }
                }
            }
            _ => return Err(invalid_sub_field("links", &sub_field, field_name)),
        }
    }

    Ok(())
}

pub fn validate_full_name_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;

    for (sub_field, sub_value) in object_entries(value) {
        let target = path(field_name, &sub_field);

        match sub_field.as_str() {
            "firstName" | "lastName" => validate_text_field(&sub_value, &target)?,
            _ => return Err(invalid_sub_field("full name", &sub_field, field_name)),
        }
    }

    Ok(())
}

pub fn validate_address_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;

    for (sub_field, sub_value) in object_entries(value) {
        let target = path(field_name, &sub_field);

        match sub_field.as_str() {
            "addressStreet1" | "addressStreet2" | "addressCity" | "addressState"
            | "addressPostcode" | "addressCountry" => validate_text_field(&sub_value, &target)?,
            "addressLat" | "addressLng" => validate_numeric_field(&sub_value, &target)?,
            _ => return Err(invalid_sub_field("address", &sub_field, field_name)),
        }
    }

    Ok(())
}

pub fn validate_currency_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;

    for (sub_field, sub_value) in object_entries(value) {
        let target = path(field_name, &sub_field);

        match sub_field.as_str() {
            "amountMicros" => validate_numeric_field(&sub_value, &target)?,
            "currencyCode" => {
                validate_text_field(&sub_value, &target)?;

                if let Some(code) = sub_value.as_str() {
                    if is_non_empty_string(&sub_value) && !CURRENCY_CODES.contains(&code) {
                        return Err(ValidationError::new(
                            format!(
                                "Invalid currency code \"{code}\" for currency field \"{field_name}\""
                            ),
                            String::from("Invalid value for currency."),
                        ));
                    }
                }
            }
            _ => return Err(invalid_sub_field("currency", &sub_field, field_name)),
        }
    }

    Ok(())
}

pub fn validate_actor_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;
    let sources: Vec<String> = ACTOR_SOURCES
        .iter()
        .map(|source| (*source).to_string())
        .collect();

    for (sub_field, sub_value) in object_entries(value) {
        let target = path(field_name, &sub_field);

        match sub_field.as_str() {
            "source" => validate_rating_and_select_field(&sub_value, &target, Some(&sources))?,
            "context" => validate_raw_json_field(&sub_value, &target)?,
            "name" => validate_text_field(&sub_value, &target)?,
            "workspaceMemberId" => validate_uuid_field(&sub_value, &target)?,
            _ => return Err(invalid_sub_field("actor", &sub_field, field_name)),
        }
    }

    Ok(())
}

fn zod_received(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn expected_string(index: usize, key: &str) -> String {
    format!("{index}.{key}: Invalid input: expected string, received undefined")
}

fn file_item_issue(item: &Value, index: usize) -> Option<String> {
    let Some(object) = item.as_object() else {
        return Some(format!(
            "{index}: Invalid input: expected object, received {}",
            zod_received(item)
        ));
    };

    if let Some(key) = object
        .keys()
        .find(|key| !matches!(key.as_str(), "fileId" | "label"))
    {
        return Some(format!("{index}: Unrecognized key: \"{key}\""));
    }

    match object.get("fileId") {
        None => return Some(expected_string(index, "fileId")),
        Some(Value::String(text)) if is_valid_uuid_v4(text) => {}
        Some(other) => {
            return Some(format!(
                "{index}.fileId: Invalid input: expected string, received {}",
                zod_received(other)
            ))
        }
    }

    match object.get("label") {
        None => Some(expected_string(index, "label")),
        Some(Value::String(_)) => None,
        Some(other) => Some(format!(
            "{index}.label: Invalid input: expected string, received {}",
            zod_received(other)
        )),
    }
}

pub fn validate_files_field(
    value: &Value,
    field_name: &str,
    max_number_of_values: Option<u64>,
) -> ValidationResult<()> {
    if is_null(value) {
        return Ok(());
    }

    let parsed_value = match value {
        Value::String(text) => match serde_json::from_str::<Value>(text) {
            Ok(parsed) => parsed,
            Err(_) => {
                let inspected = inspect(value);

                return Err(ValidationError::new(
                    format!(
                        "Invalid value \"{inspected}\" for FILES field \"{field_name}\" - It should be an array of objects with \"fileId\" and \"label\" properties."
                    ),
                    format!(
                        "Invalid value \"{inspected}\" for FILES field \"{field_name}\" - It should be an array of objects with \"fileId\" and \"label\" properties."
                    ),
                ));
            }
        },
        other => other.clone(),
    };

    let items = match &parsed_value {
        Value::Array(items) => items.clone(),
        other => {
            let details = format!(
                "Invalid input: expected array, received {}",
                zod_received(other)
            );

            return Err(files_schema_error(field_name, &parsed_value, &details));
        }
    };

    let issues: Vec<String> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| file_item_issue(item, index))
        .collect();

    if !issues.is_empty() {
        return Err(files_schema_error(
            field_name,
            &parsed_value,
            &issues.join(", "),
        ));
    }

    if let Some(maximum) = max_number_of_values {
        if (items.len() as u64) > maximum {
            return Err(ValidationError::new(
                format!("Max number of files is {maximum} for field \"{field_name}\""),
                format!("Max number of files is {maximum} for field \"{field_name}\""),
            ));
        }
    }

    Ok(())
}

fn files_schema_error(field_name: &str, parsed: &Value, details: &str) -> ValidationError {
    ValidationError::new(
        format!(
            "Invalid value \"{}\" for FILES field \"{field_name}\" - {details}",
            inspect(parsed)
        ),
        format!("Invalid value for FILES field \"{field_name}\" - {details}"),
    )
}

fn is_dangerous_url(value: &Value) -> bool {
    if !is_string(value) {
        return false;
    }

    let text = value.as_str().unwrap_or_default().trim();

    !text.is_empty() && !is_safe_url(text)
}

fn has_dangerous_blocknote_url(value: &Value) -> bool {
    if let Value::Array(items) = value {
        return items.iter().any(has_dangerous_blocknote_url);
    }

    if !is_plain_object(value) {
        return false;
    }

    let Some(object) = value.as_object() else {
        return false;
    };

    object.iter().any(|(key, entry)| match key.as_str() {
        "url" | "href" | "src" => is_dangerous_url(entry),
        _ => has_dangerous_blocknote_url(entry),
    })
}

fn rich_text_error(reason: String) -> ValidationError {
    ValidationError::standard(reason)
}

fn validate_blocknote_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_text_field(value, field_name)?;

    let Some(text) = value.as_str() else {
        return Ok(());
    };

    if text.is_empty() {
        return Ok(());
    }

    let parsed = serde_json::from_str::<Value>(text).map_err(|_| {
        rich_text_error(format!(
            "Invalid blocknote value for field \"{field_name}\" - must contain valid JSON"
        ))
    })?;

    if !parsed.is_array() {
        return Err(rich_text_error(format!(
            "Invalid blocknote value for field \"{field_name}\" - must be a JSON array of blocks"
        )));
    }

    if has_dangerous_blocknote_url(&parsed) {
        return Err(ValidationError::new(
            format!("Dangerous URL protocol in blocknote content for field \"{field_name}\""),
            String::from("Content contains a URL with a dangerous protocol."),
        ));
    }

    Ok(())
}

fn invalid_url_reason(value: &Value) -> Option<&'static str> {
    let text = value.as_str()?;

    if text.chars().count() > URL_MAX_LENGTH {
        return Some("Too big: expected string to be less than or equal to 4000 characters");
    }

    let trimmed = text.trim();

    if trimmed.is_empty() || is_safe_url(trimmed) {
        return None;
    }

    Some("Invalid input")
}

fn optional_int(value: &Value, minimum: u64, maximum: u64) -> bool {
    match value {
        Value::Null => true,
        Value::Number(number) => match number.as_f64() {
            Some(inner) if inner.fract() == 0.0 && inner >= 0.0 => {
                inner >= minimum as f64 && inner <= maximum as f64
            }
            _ => false,
        },
        _ => false,
    }
}

fn valid_text_alignments(attrs: Option<&Value>) -> bool {
    let Some(attrs) = attrs.and_then(Value::as_object) else {
        return true;
    };

    match attrs.get("textAlign") {
        None | Some(Value::Null) => true,
        Some(value) => value
            .as_str()
            .is_some_and(|text| TEXT_ALIGNMENTS.contains(&text)),
    }
}

fn valid_indent(attrs: &Value) -> bool {
    let Some(object) = attrs.as_object() else {
        return false;
    };

    match object.get("indent") {
        None | Some(Value::Null) => true,
        Some(value) => optional_int(value, 0, MAX_INDENT_LEVEL),
    }
}

fn valid_block_attrs(attrs: Option<&Value>) -> bool {
    match attrs {
        None | Some(Value::Null) => true,
        Some(attrs) => valid_indent(attrs) && valid_text_alignments(Some(attrs)),
    }
}

fn valid_inline_content(value: &Value) -> bool {
    let Some(items) = value.as_array() else {
        return false;
    };

    items.iter().all(validate_inline_node)
}

fn validate_inline_node(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    match object.get("type").and_then(Value::as_str) {
        Some("text") => {
            let valid_text = object
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.is_empty());

            valid_text && object.get("marks").map(validate_marks).unwrap_or(true)
        }
        Some("hardBreak") => true,
        Some("mentionTag") => {
            object
                .get("attrs")
                .and_then(Value::as_object)
                .map(|attrs| {
                    matches!(attrs.get("recordId"), None | Some(Value::Null))
                        || attrs.get("recordId").is_some_and(|value| value.is_string())
                })
                .unwrap_or(false)
                && matches!(
                    object
                        .get("attrs")
                        .and_then(|attrs| attrs.get("objectNameSingular")),
                    None | Some(Value::Null) | Some(Value::String(_))
                )
        }
        Some(other) => INLINE_NODE_TYPES.contains(&other),
        None => false,
    }
}

fn validate_marks(value: &Value) -> bool {
    let Some(items) = value.as_array() else {
        return false;
    };

    items.iter().all(|mark| {
        let Some(object) = mark.as_object() else {
            return false;
        };

        match object.get("type").and_then(Value::as_str) {
            Some("link") => object
                .get("attrs")
                .and_then(|attrs| attrs.get("href"))
                .map(|href| href.is_string() && invalid_url_reason(href).is_none())
                .unwrap_or(false),
            Some("textStyle") => {
                for key in ["textColor", "backgroundColor"] {
                    match object.get("attrs").and_then(|attrs| attrs.get(key)) {
                        None | Some(Value::Null) => {}
                        Some(value) => {
                            if !value.as_str().is_some_and(|text| COLORS.contains(&text)) {
                                return false;
                            }
                        }
                    }
                }

                true
            }
            Some(other) => MARK_TYPES.contains(&other),
            None => false,
        }
    })
}

fn valid_block_content(value: Option<&Value>, required: bool) -> bool {
    match value {
        None => !required,
        Some(value) => {
            let Some(items) = value.as_array() else {
                return false;
            };

            if required && items.is_empty() {
                return false;
            }

            items.iter().all(validate_block_node)
        }
    }
}

fn validate_block_node(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    let Some(kind) = object.get("type").and_then(Value::as_str) else {
        return false;
    };

    if !BLOCK_NODE_TYPES.contains(&kind) {
        return false;
    }

    let attrs = object.get("attrs");

    match kind {
        "paragraph" => {
            valid_block_attrs(attrs.cloned().as_ref().or(None))
                && valid_block_content(None, false)
                && object
                    .get("content")
                    .map(valid_inline_content)
                    .unwrap_or(true)
        }
        "heading" => {
            let level_ok = attrs
                .and_then(|attrs| attrs.get("level"))
                .map(|level| optional_int(level, 1, MAX_HEADING_LEVEL))
                .unwrap_or(false);

            level_ok && valid_block_content(object.get("content"), false)
        }
        "bulletList" | "taskList" | "blockquote" => {
            let min_one = object
                .get("content")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty());

            min_one && valid_block_content(object.get("content"), true)
        }
        "orderedList" => {
            object
                .get("content")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
                && valid_block_content(object.get("content"), true)
        }
        "divider" => true,
        "image" => object
            .get("attrs")
            .and_then(|attrs| attrs.get("src"))
            .map(|src| src.is_string() && invalid_url_reason(src).is_none())
            .unwrap_or(false),
        "file" => object
            .get("attrs")
            .and_then(|attrs| attrs.get("url"))
            .map(|url| url.is_string() && invalid_url_reason(url).is_none())
            .unwrap_or(false),
        "table" => object
            .get("content")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty() && rows.iter().all(validate_table_row))
            .unwrap_or(false),
        "toggle" => {
            let content = object.get("content").and_then(Value::as_array);

            match content {
                Some(items) if items.len() >= 2 => {
                    let first = items[0].as_object().and_then(|node| node.get("type"))
                        == Some(&Value::String(String::from("paragraph")))
                        || items[0].as_object().and_then(|node| node.get("type"))
                            == Some(&Value::String(String::from("heading")));
                    let tail_ok = items[1..].iter().all(validate_block_node);

                    first && tail_ok
                }
                _ => false,
            }
        }
        "codeBlock" => object
            .get("content")
            .map(|content| {
                content.as_array().is_some_and(|items| {
                    items.iter().all(|item| {
                        item.get("type").and_then(Value::as_str) == Some("text")
                            && item
                                .get("text")
                                .and_then(Value::as_str)
                                .is_some_and(|text| !text.is_empty())
                    })
                })
            })
            .unwrap_or(true),
        _ => valid_block_content(object.get("content"), false),
    }
}

fn validate_table_row(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("tableRow")
        && value
            .get("content")
            .and_then(Value::as_array)
            .map(|cells| {
                !cells.is_empty()
                    && cells.iter().all(|cell| {
                        matches!(
                            cell.get("type").and_then(Value::as_str),
                            Some("tableCell") | Some("tableHeader")
                        ) && cell
                            .get("content")
                            .and_then(Value::as_array)
                            .is_some_and(|items| !items.is_empty())
                            && cell
                                .get("content")
                                .map(|content| valid_block_content(Some(content), false))
                                .unwrap_or(false)
                    })
            })
            .unwrap_or(false)
}

fn validate_tiptap_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_text_field(value, field_name)?;

    let Some(text) = value.as_str() else {
        return Ok(());
    };

    if text.is_empty() {
        return Ok(());
    }

    let parsed = serde_json::from_str::<Value>(text).unwrap_or(Value::Null);
    let reason = match &parsed {
        Value::Object(object) if object.is_empty() => Some(String::from(
            "type: Invalid input: expected \"doc\", received undefined",
        )),
        Value::Object(object) if !object.contains_key("type") => Some(String::from(
            "type: Invalid input: expected \"doc\", received undefined",
        )),
        Value::Object(object) if object.get("type").and_then(Value::as_str) == Some("doc") => {
            match object.get("content") {
                None => None,
                Some(content) => {
                    let valid = content
                        .as_array()
                        .map(|items| items.iter().all(validate_block_node))
                        .unwrap_or(false);

                    if valid {
                        None
                    } else {
                        Some(String::from(
                            "content.0: Invalid discriminated union property",
                        ))
                    }
                }
            }
        }
        Value::Object(_) => {
            Some("type: Invalid input: expected \"doc\", received string".to_string())
        }
        other => Some(format!(
            "Invalid input: expected object, received {}",
            zod_received(other)
        )),
    };

    match reason {
        None => Ok(()),
        Some(reason) => Err(ValidationError::standard(format!(
            "Invalid tiptap value for field \"{field_name}\" - {reason}"
        ))),
    }
}

pub fn validate_rich_text_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    validate_raw_json_field(value, field_name)?;

    for (sub_field, sub_value) in object_entries(value) {
        let target = path(field_name, &sub_field);

        match sub_field.as_str() {
            "blocknote" => validate_blocknote_field(&sub_value, &target)?,
            "markdown" => validate_text_field(&sub_value, &target)?,
            "tiptap" => validate_tiptap_field(&sub_value, &target)?,
            _ => {
                return Err(rich_text_error(format!(
                    "Invalid subfield {} for rich text field \"{field_name}\"",
                    inspect(&Value::String(sub_field))
                )))
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn failure(result: ValidationResult<()>) -> (String, String) {
        let error = result.expect_err("expected a validation error");

        (
            error.message().to_string(),
            error.user_friendly_message().to_string(),
        )
    }

    #[test]
    fn emails_reports_the_offending_sub_path() {
        assert!(validate_emails_field(&json!({ "primaryEmail": "a@b.co" }), "emails").is_ok());
        assert!(validate_emails_field(&Value::Null, "emails").is_ok());
        assert_eq!(
            failure(validate_emails_field(
                &json!({ "primaryEmail": "nope" }),
                "emails"
            )),
            (
                "Invalid string value 'nope' for email field \"emails.primaryEmail\"".to_string(),
                "Invalid value: \"'nope'\"".to_string()
            )
        );
    }

    #[test]
    fn emails_rejects_unknown_subfields_and_null_items() {
        assert_eq!(
            failure(validate_emails_field(
                &json!({ "primary": "a@b.co" }),
                "emails"
            ))
            .0,
            "Invalid subfield primary for emails field \"emails\""
        );
        assert_eq!(
            failure(validate_emails_field(
                &json!({ "additionalEmails": ["a@b.co", null] }),
                "emails"
            ))
            .0,
            "Invalid value [ 'a@b.co', null ] for field \"emails.additionalEmails - Array values need to be string\""
        );
    }

    #[test]
    fn phones_validates_sub_fields_as_text() {
        assert!(validate_phones_field(&json!({ "primaryPhoneNumber": 5 }), "phones").is_err());
        assert_eq!(
            failure(validate_phones_field(&json!({ "whatever": "x" }), "phones")).0,
            "Invalid subfield whatever for phones field \"phones\""
        );
        assert_eq!(
            failure(validate_phones_field(&json!({ "whatever": "x" }), "phones")).1,
            "Invalid value for phones."
        );
    }

    #[test]
    fn links_check_domains_only_for_the_domain_variant() {
        assert!(validate_links_field(&json!({ "primaryLinkUrl": "hello" }), "links", None).is_ok());
        assert_eq!(
            failure(validate_links_field(
                &json!({ "primaryLinkUrl": "not a domain" }),
                "links",
                Some("domain")
            ))
            .0,
            "\"not a domain\" is not a domain name, for domain-typed links field \"links.primaryLinkUrl\""
        );
        assert!(validate_links_field(
            &json!({ "primaryLinkUrl": "twenty.com" }),
            "links",
            Some("domain")
        )
        .is_ok());
        assert_eq!(
            failure(validate_links_field(
                &json!({ "secondaryLinks": [{ "url": "bad url" }] }),
                "links",
                Some("domain")
            ))
            .0,
            "\"bad url\" is not a domain name, for domain-typed links field \"links.secondaryLinks\""
        );
    }

    #[test]
    fn full_name_and_address_reject_unknown_subfields() {
        assert_eq!(
            failure(validate_full_name_field(&json!({ "middle": "x" }), "name")).0,
            "Invalid subfield middle for full name field \"name\""
        );
        assert_eq!(
            failure(validate_address_field(
                &json!({ "addressZip": "x" }),
                "address"
            ))
            .0,
            "Invalid subfield addressZip for address field \"address\""
        );
        assert_eq!(
            failure(validate_address_field(
                &json!({ "addressLat": "nope" }),
                "address"
            ))
            .0,
            "Invalid number value NaN for field \"address.addressLat\""
        );
    }

    #[test]
    fn currency_validates_the_code_and_the_amount() {
        assert!(validate_currency_field(
            &json!({ "amountMicros": 1000, "currencyCode": "USD" }),
            "amount"
        )
        .is_ok());
        assert_eq!(
            failure(validate_currency_field(
                &json!({ "currencyCode": "XYZ" }),
                "amount"
            ))
            .0,
            "Invalid currency code \"XYZ\" for currency field \"amount\""
        );
        assert_eq!(
            failure(validate_currency_field(&json!({ "value": 1 }), "amount")).0,
            "Invalid subfield value for currency field \"amount\""
        );
    }

    #[test]
    fn actor_source_must_be_a_known_source() {
        assert!(
            validate_actor_field(&json!({ "source": "MANUAL", "name": "Tony" }), "actor").is_ok()
        );
        assert_eq!(
            failure(validate_actor_field(&json!({ "source": "ROBOT" }), "actor")).0,
            "Invalid value \"ROBOT\" for field \"actor.source\". Valid values are: EMAIL, CALENDAR, WORKFLOW, AGENT, API, IMPORT, MANUAL, SYSTEM, WEBHOOK, APPLICATION"
        );
    }

    #[test]
    fn files_requires_object_id_and_label_pairs() {
        assert!(validate_files_field(
            &json!([{ "fileId": "20202020-0000-4000-8000-000000000001", "label": "a.pdf" }]),
            "files",
            Some(1)
        )
        .is_ok());
        assert_eq!(
            failure(validate_files_field(
                &json!([{ "fileId": "nope", "label": "a.pdf" }]),
                "files",
                Some(1)
            ))
            .1,
            "Invalid value for FILES field \"files\" - 0.fileId: Invalid input: expected string, received string"
        );
        assert_eq!(
            failure(validate_files_field(
                &json!([{ "fileId": "20202020-0000-4000-8000-000000000001", "label": "a.pdf" }, { "fileId": "20202020-0000-4000-8000-000000000002", "label": "b.pdf" }]),
                "files",
                Some(1)
            ))
            .0,
            "Max number of files is 1 for field \"files\""
        );
        assert!(validate_files_field(&json!("not json"), "files", Some(1)).is_err());
    }

    #[test]
    fn rich_text_flags_unknown_subfields_and_dangerous_urls() {
        assert_eq!(
            failure(validate_rich_text_field(&json!({ "html": "x" }), "note")).0,
            "Invalid subfield 'html' for rich text field \"note\""
        );
        assert!(validate_rich_text_field(
            &json!({ "blocknote": "[{\"id\":\"1\",\"prop\":\"url\",\"content\":\"javascript:alert(1)\"}]" }),
            "note"
        )
        .is_err());
        assert!(validate_rich_text_field(&json!({ "blocknote": "not-json" }), "note").is_err());
        assert!(validate_rich_text_field(&json!({ "blocknote": "{ \"a\": 1 }" }), "note").is_err());
        assert!(
            validate_rich_text_field(&json!({ "tiptap": "{\"type\":\"paragraph\"}" }), "note")
                .is_err()
        );
        assert!(validate_rich_text_field(
            &json!({ "tiptap": "{\"type\":\"doc\",\"content\":[{\"type\":\"paragraph\",\"content\":[{\"type\":\"text\",\"text\":\"hi\"}]}]}" }),
            "note"
        )
        .is_ok());
    }
}
