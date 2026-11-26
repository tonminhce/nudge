#![allow(dead_code)]

pub const TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER: &str =
    "20202020-64aa-4b6f-b003-9c74b97cee20";

pub const IDENTIFIER_MIN_CHAR_LENGTH: usize = 1;
pub const IDENTIFIER_MAX_CHAR_LENGTH: usize = 63;

pub const CUSTOM_NAME_PREFIX: char = '_';

pub const RESERVED_METADATA_NAME_KEYWORDS: [&str; 70] = [
    "approvedAccessDomain",
    "approvedAccessDomains",
    "appToken",
    "appTokens",
    "billingCustomer",
    "billingCustomers",
    "billingEntitlement",
    "billingEntitlements",
    "billingMeter",
    "billingMeters",
    "billingProduct",
    "billingProducts",
    "billingSubscription",
    "billingSubscriptions",
    "billingSubscriptionItem",
    "billingSubscriptionItems",
    "featureFlag",
    "featureFlags",
    "job",
    "jobs",
    "keyValuePair",
    "keyValuePairs",
    "pageLayout",
    "pageLayouts",
    "pageLayoutTab",
    "pageLayoutTabs",
    "pageLayoutWidget",
    "pageLayoutWidgets",
    "twoFactorMethod",
    "twoFactorMethods",
    "user",
    "users",
    "userWorkspace",
    "userWorkspaces",
    "workspace",
    "workspaces",
    "role",
    "roles",
    "userWorkspaceRole",
    "userWorkspaceRoles",
    "plan",
    "plans",
    "event",
    "events",
    "field",
    "fields",
    "link",
    "links",
    "currency",
    "currencies",
    "fullNames",
    "address",
    "addresses",
    "type",
    "types",
    "object",
    "objects",
    "index",
    "relation",
    "relations",
    "aggregate",
    "connect",
    "create",
    "disconnect",
    "search",
    "searches",
    "workflow",
    "workflows",
    "workflowVersion",
    "workflowVersions",
];

pub const OBJECT_NAME_INVALID_INPUT_CODE: &str = "INVALID_OBJECT_INPUT";
pub const FIELD_NAME_INVALID_INPUT_CODE: &str = "INVALID_FIELD_INPUT";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataValidationError {
    pub code: &'static str,
    pub message: String,
    pub value: Option<String>,
}

impl MetadataValidationError {
    fn new(code: &'static str, message: impl Into<String>, value: Option<&str>) -> Self {
        Self {
            code,
            message: message.into(),
            value: value.map(str::to_string),
        }
    }
}

impl std::fmt::Display for MetadataValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for MetadataValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataNameError {
    pub message: String,
}

impl std::fmt::Display for MetadataNameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for MetadataNameError {}

pub fn compute_table_name(name_singular: &str, is_custom: bool) -> String {
    if is_custom {
        format!("{CUSTOM_NAME_PREFIX}{name_singular}")
    } else {
        name_singular.to_string()
    }
}

pub fn compute_object_target_table(
    name_singular: &str,
    application_universal_identifier: &str,
) -> String {
    compute_table_name(
        name_singular,
        application_universal_identifier != TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER,
    )
}

pub fn compute_column_name(field_name: &str) -> String {
    field_name.to_string()
}

pub fn compute_relation_join_column_name(field_name: &str) -> String {
    format!("{field_name}Id")
}

pub fn pascal_case(value: &str) -> String {
    capitalize(&camel_case(value))
}

pub fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

const DEBURRED_LETTERS: &[(char, &str)] = &[
    ('\u{00c0}', "A"),
    ('\u{00c1}', "A"),
    ('\u{00c2}', "A"),
    ('\u{00c3}', "A"),
    ('\u{00c4}', "A"),
    ('\u{00c5}', "A"),
    ('\u{00c6}', "Ae"),
    ('\u{00c7}', "C"),
    ('\u{00c8}', "E"),
    ('\u{00c9}', "E"),
    ('\u{00ca}', "E"),
    ('\u{00cb}', "E"),
    ('\u{00cc}', "I"),
    ('\u{00cd}', "I"),
    ('\u{00ce}', "I"),
    ('\u{00cf}', "I"),
    ('\u{00d0}', "D"),
    ('\u{00d1}', "N"),
    ('\u{00d2}', "O"),
    ('\u{00d3}', "O"),
    ('\u{00d4}', "O"),
    ('\u{00d5}', "O"),
    ('\u{00d6}', "O"),
    ('\u{00d8}', "O"),
    ('\u{00d9}', "U"),
    ('\u{00da}', "U"),
    ('\u{00db}', "U"),
    ('\u{00dc}', "U"),
    ('\u{00dd}', "Y"),
    ('\u{00de}', "Th"),
    ('\u{00df}', "ss"),
    ('\u{00e0}', "a"),
    ('\u{00e1}', "a"),
    ('\u{00e2}', "a"),
    ('\u{00e3}', "a"),
    ('\u{00e4}', "a"),
    ('\u{00e5}', "a"),
    ('\u{00e6}', "ae"),
    ('\u{00e7}', "c"),
    ('\u{00e8}', "e"),
    ('\u{00e9}', "e"),
    ('\u{00ea}', "e"),
    ('\u{00eb}', "e"),
    ('\u{00ec}', "i"),
    ('\u{00ed}', "i"),
    ('\u{00ee}', "i"),
    ('\u{00ef}', "i"),
    ('\u{00f0}', "d"),
    ('\u{00f1}', "n"),
    ('\u{00f2}', "o"),
    ('\u{00f3}', "o"),
    ('\u{00f4}', "o"),
    ('\u{00f5}', "o"),
    ('\u{00f6}', "o"),
    ('\u{00f8}', "o"),
    ('\u{00f9}', "u"),
    ('\u{00fa}', "u"),
    ('\u{00fb}', "u"),
    ('\u{00fc}', "u"),
    ('\u{00fd}', "y"),
    ('\u{00fe}', "th"),
    ('\u{00ff}', "y"),
    ('\u{0100}', "A"),
    ('\u{0101}', "a"),
    ('\u{0102}', "A"),
    ('\u{0103}', "a"),
    ('\u{0104}', "A"),
    ('\u{0105}', "a"),
    ('\u{0106}', "C"),
    ('\u{0107}', "c"),
    ('\u{0108}', "C"),
    ('\u{0109}', "c"),
    ('\u{010a}', "C"),
    ('\u{010b}', "c"),
    ('\u{010c}', "C"),
    ('\u{010d}', "c"),
    ('\u{010e}', "D"),
    ('\u{010f}', "d"),
    ('\u{0110}', "D"),
    ('\u{0111}', "d"),
    ('\u{0112}', "E"),
    ('\u{0113}', "e"),
    ('\u{0114}', "E"),
    ('\u{0115}', "e"),
    ('\u{0116}', "E"),
    ('\u{0117}', "e"),
    ('\u{0118}', "E"),
    ('\u{0119}', "e"),
    ('\u{011a}', "E"),
    ('\u{011b}', "e"),
    ('\u{011c}', "G"),
    ('\u{011d}', "g"),
    ('\u{011e}', "G"),
    ('\u{011f}', "g"),
    ('\u{0120}', "G"),
    ('\u{0121}', "g"),
    ('\u{0122}', "G"),
    ('\u{0123}', "g"),
    ('\u{0124}', "H"),
    ('\u{0125}', "h"),
    ('\u{0126}', "H"),
    ('\u{0127}', "h"),
    ('\u{0128}', "I"),
    ('\u{0129}', "i"),
    ('\u{012a}', "I"),
    ('\u{012b}', "i"),
    ('\u{012c}', "I"),
    ('\u{012d}', "i"),
    ('\u{012e}', "I"),
    ('\u{012f}', "i"),
    ('\u{0130}', "I"),
    ('\u{0131}', "i"),
    ('\u{0132}', "IJ"),
    ('\u{0133}', "ij"),
    ('\u{0134}', "J"),
    ('\u{0135}', "j"),
    ('\u{0136}', "K"),
    ('\u{0137}', "k"),
    ('\u{0138}', "k"),
    ('\u{0139}', "L"),
    ('\u{013a}', "l"),
    ('\u{013b}', "L"),
    ('\u{013c}', "l"),
    ('\u{013d}', "L"),
    ('\u{013e}', "l"),
    ('\u{013f}', "L"),
    ('\u{0140}', "l"),
    ('\u{0141}', "L"),
    ('\u{0142}', "l"),
    ('\u{0143}', "N"),
    ('\u{0144}', "n"),
    ('\u{0145}', "N"),
    ('\u{0146}', "n"),
    ('\u{0147}', "N"),
    ('\u{0148}', "n"),
    ('\u{0149}', "'n"),
    ('\u{014a}', "N"),
    ('\u{014b}', "n"),
    ('\u{014c}', "O"),
    ('\u{014d}', "o"),
    ('\u{014e}', "O"),
    ('\u{014f}', "o"),
    ('\u{0150}', "O"),
    ('\u{0151}', "o"),
    ('\u{0152}', "Oe"),
    ('\u{0153}', "oe"),
    ('\u{0154}', "R"),
    ('\u{0155}', "r"),
    ('\u{0156}', "R"),
    ('\u{0157}', "r"),
    ('\u{0158}', "R"),
    ('\u{0159}', "r"),
    ('\u{015a}', "S"),
    ('\u{015b}', "s"),
    ('\u{015c}', "S"),
    ('\u{015d}', "s"),
    ('\u{015e}', "S"),
    ('\u{015f}', "s"),
    ('\u{0160}', "S"),
    ('\u{0161}', "s"),
    ('\u{0162}', "T"),
    ('\u{0163}', "t"),
    ('\u{0164}', "T"),
    ('\u{0165}', "t"),
    ('\u{0166}', "T"),
    ('\u{0167}', "t"),
    ('\u{0168}', "U"),
    ('\u{0169}', "u"),
    ('\u{016a}', "U"),
    ('\u{016b}', "u"),
    ('\u{016c}', "U"),
    ('\u{016d}', "u"),
    ('\u{016e}', "U"),
    ('\u{016f}', "u"),
    ('\u{0170}', "U"),
    ('\u{0171}', "u"),
    ('\u{0172}', "U"),
    ('\u{0173}', "u"),
    ('\u{0174}', "W"),
    ('\u{0175}', "w"),
    ('\u{0176}', "Y"),
    ('\u{0177}', "y"),
    ('\u{0178}', "Y"),
    ('\u{0179}', "Z"),
    ('\u{017a}', "z"),
    ('\u{017b}', "Z"),
    ('\u{017c}', "z"),
    ('\u{017d}', "Z"),
    ('\u{017e}', "z"),
    ('\u{017f}', "ss"),
];

fn deburr_char(character: char) -> Option<&'static str> {
    DEBURRED_LETTERS
        .binary_search_by_key(&character, |entry| entry.0)
        .ok()
        .map(|index| DEBURRED_LETTERS[index].1)
}

fn find_word_starting_at(chars: &[char], start: usize) -> Option<usize> {
    let remaining = &chars[start..];

    if let Some(length) = find_lower_word_with_lookahead(remaining) {
        return Some(length);
    }

    if let Some(length) = find_upper_word_with_lookahead(remaining) {
        return Some(length);
    }

    if let Some(length) = find_lower_word_plain(remaining) {
        return Some(length);
    }

    if let Some(length) = find_upper_word_plain(remaining) {
        return Some(length);
    }

    if remaining[0].is_ascii_digit() {
        let mut length = 1;
        while length < remaining.len() && remaining[length].is_ascii_digit() {
            length += 1;
        }
        return Some(length);
    }

    None
}

fn find_lower_word_plain(chars: &[char]) -> Option<usize> {
    let mut index = 0;

    if chars[0].is_ascii_uppercase() {
        index += 1;
        if index == chars.len() || !chars[index].is_ascii_lowercase() {
            return None;
        }
    } else if !chars[0].is_ascii_lowercase() {
        return None;
    }

    let mut end = index;
    while end < chars.len() && chars[end].is_ascii_lowercase() {
        end += 1;
    }

    Some(end)
}

fn find_lower_word_with_lookahead(chars: &[char]) -> Option<usize> {
    let end = find_lower_word_plain(chars)?;

    if end == chars.len() {
        return Some(end);
    }

    let next = chars[end];
    if !next.is_ascii_alphanumeric() || next.is_ascii_uppercase() {
        return Some(end);
    }

    None
}

fn find_upper_word_with_lookahead(chars: &[char]) -> Option<usize> {
    let max_run = find_upper_word_plain(chars)?;

    let mut end = max_run;

    loop {
        if end == chars.len() {
            return Some(end);
        }

        let next = chars[end];

        if !next.is_ascii_alphanumeric() {
            return Some(end);
        }

        if next.is_ascii_uppercase() && end + 1 < chars.len() && chars[end + 1].is_ascii_lowercase()
        {
            return Some(end);
        }

        if end == 1 {
            return None;
        }

        end -= 1;
    }
}

fn find_upper_word_plain(chars: &[char]) -> Option<usize> {
    if !chars[0].is_ascii_uppercase() {
        return None;
    }

    let mut end = 1;
    while end < chars.len() && chars[end].is_ascii_uppercase() {
        end += 1;
    }

    Some(end)
}

pub fn camel_case(value: &str) -> String {
    let deburred: Vec<char> = value
        .chars()
        .flat_map(|character| match deburr_char(character) {
            Some(expansion) => expansion.chars().collect(),
            None => vec![character],
        })
        .collect();

    let mut words: Vec<String> = Vec::new();
    let mut index = 0;

    while index < deburred.len() {
        match find_word_starting_at(&deburred, index) {
            Some(length) => {
                let word: String = deburred[index..index + length].iter().collect();
                words.push(word.to_lowercase());
                index += length;
            }
            None => index += 1,
        }
    }

    let mut result = String::new();
    for (position, word) in words.iter().enumerate() {
        if position == 0 {
            result.push_str(word);
        } else {
            result.push_str(&capitalize(word));
        }
    }

    result
}

pub fn compute_composite_column_name(field_name: &str, property_name: &str) -> String {
    format!("{field_name}{}", pascal_case(property_name))
}

pub fn compute_server_only_composite_column_name(field_name: &str, column_name: &str) -> String {
    format!("{field_name}_{column_name}")
}

pub fn remove_sql_ddl_injection(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect()
}

pub fn compute_postgres_enum_name(table_name: &str, column_name: &str) -> String {
    remove_sql_ddl_injection(&format!("{table_name}_{column_name}_enum"))
}

fn fold_to_slug_characters(label: &str) -> String {
    let mut folded = String::new();

    for character in label.chars() {
        if let Some(expansion) = deburr_char(character) {
            folded.push_str(expansion);
        } else if character.is_ascii() {
            folded.push(character);
        }
    }

    folded
}

pub fn slugify(label: &str) -> String {
    let folded = fold_to_slug_characters(label).to_lowercase();

    let mut separator_replaced = String::with_capacity(folded.len());
    let mut previous_was_separator = false;

    for character in folded.chars() {
        if character.is_ascii_alphanumeric() {
            separator_replaced.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator {
            separator_replaced.push('_');
            previous_was_separator = true;
        }
    }

    separator_replaced.trim_matches('_').to_string()
}

pub fn add_custom_suffix_if_is_reserved(name: &str) -> String {
    if name.is_empty() {
        return name.to_string();
    }

    if RESERVED_METADATA_NAME_KEYWORDS.contains(&name) {
        format!("{name}Custom")
    } else {
        name.to_string()
    }
}

pub fn compute_metadata_name_from_label(
    label: &str,
    apply_custom_suffix: bool,
) -> Result<String, MetadataNameError> {
    if label.is_empty() {
        return Ok(String::new());
    }

    let prefixed_label = if label.starts_with(|character: char| character.is_ascii_digit()) {
        format!("n{label}")
    } else {
        label.to_string()
    };

    let formatted_string = slugify(&prefixed_label);

    if formatted_string.is_empty() {
        return Err(MetadataNameError {
            message: format!("Invalid label: \"{label}\""),
        });
    }

    let computed_name = camel_case(&formatted_string);

    if apply_custom_suffix {
        Ok(add_custom_suffix_if_is_reserved(&computed_name))
    } else {
        Ok(computed_name)
    }
}

fn is_valid_object_name_format(name: &str) -> bool {
    let mut chars = name.chars();

    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }

    chars.all(|character| character.is_ascii_alphanumeric())
}

pub fn validate_object_metadata_names(
    name_singular: &str,
    name_plural: &str,
) -> Vec<MetadataValidationError> {
    let mut errors = Vec::new();

    for name in [name_singular, name_plural] {
        if name.chars().count() > IDENTIFIER_MAX_CHAR_LENGTH {
            errors.push(MetadataValidationError::new(
                OBJECT_NAME_INVALID_INPUT_CODE,
                "Name is too long",
                Some(name),
            ));
        }

        if name.chars().count() < IDENTIFIER_MIN_CHAR_LENGTH {
            errors.push(MetadataValidationError::new(
                OBJECT_NAME_INVALID_INPUT_CODE,
                "Name is too short",
                Some(name),
            ));
        }

        if !is_valid_object_name_format(name) {
            errors.push(MetadataValidationError::new(
                OBJECT_NAME_INVALID_INPUT_CODE,
                "Name is not valid: it must start with lowercase letter and contain only alphanumeric letters",
                Some(name),
            ));
        } else {
            let camel_cased_name = camel_case(name);

            if camel_cased_name != name {
                errors.push(MetadataValidationError::new(
                    OBJECT_NAME_INVALID_INPUT_CODE,
                    format!("Name is not valid: it must be camelCase, e.g. \"{camel_cased_name}\""),
                    Some(name),
                ));
            }
        }

        if RESERVED_METADATA_NAME_KEYWORDS.contains(&name) {
            errors.push(MetadataValidationError::new(
                OBJECT_NAME_INVALID_INPUT_CODE,
                "This name is reserved. Use a different name or the system will add \"Custom\" suffix.",
                Some(name),
            ));
        }
    }

    if name_plural.trim().to_lowercase() == name_singular.trim().to_lowercase() {
        errors.push(MetadataValidationError::new(
            OBJECT_NAME_INVALID_INPUT_CODE,
            "The singular and plural names cannot be the same for an object",
            Some(name_plural),
        ));
    }

    errors
}

pub fn validate_field_metadata_name(
    name: &str,
    is_standard_app_caller: bool,
) -> Vec<MetadataValidationError> {
    let mut errors = Vec::new();

    if name.chars().count() > IDENTIFIER_MAX_CHAR_LENGTH {
        errors.push(MetadataValidationError::new(
            FIELD_NAME_INVALID_INPUT_CODE,
            "Name is too long",
            Some(name),
        ));
    }

    if name.chars().count() < IDENTIFIER_MIN_CHAR_LENGTH {
        errors.push(MetadataValidationError::new(
            FIELD_NAME_INVALID_INPUT_CODE,
            "Name is too short",
            Some(name),
        ));
    }

    if !is_valid_object_name_format(name) {
        errors.push(MetadataValidationError::new(
            FIELD_NAME_INVALID_INPUT_CODE,
            "Name is not valid: it must start with lowercase letter and contain only alphanumeric letters",
            Some(name),
        ));
    }

    if !is_standard_app_caller && RESERVED_METADATA_NAME_KEYWORDS.contains(&name) {
        errors.push(MetadataValidationError::new(
            FIELD_NAME_INVALID_INPUT_CODE,
            "This name is reserved. Use a different name or the system will add \"Custom\" suffix.",
            Some(name),
        ));
    }

    errors
}

pub fn does_other_object_with_same_name_exists(
    name_singular: &str,
    name_plural: &str,
    existing_object_universal_identifier: Option<&str>,
    existing_objects: &[(String, String, String)],
) -> bool {
    existing_objects
        .iter()
        .any(|(universal_identifier, other_singular, other_plural)| {
            (other_singular == name_singular
                || other_plural == name_plural
                || other_singular == name_plural
                || other_plural == name_singular)
                && Some(universal_identifier.as_str()) != existing_object_universal_identifier
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_name_uses_the_underscore_prefix_for_custom_objects() {
        assert_eq!(compute_table_name("person", false), "person");
        assert_eq!(compute_table_name("person", true), "_person");
        assert_eq!(
            compute_table_name("workspaceMember", true),
            "_workspaceMember"
        );
    }

    #[test]
    fn object_target_table_depends_on_the_standard_application_identifier() {
        assert_eq!(
            compute_object_target_table("person", TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER),
            "person"
        );
        assert_eq!(compute_object_target_table("rocket", "app-1"), "_rocket");
    }

    #[test]
    fn composite_column_names_follow_the_data_model_examples() {
        assert_eq!(
            compute_composite_column_name("name", "firstName"),
            "nameFirstName"
        );
        assert_eq!(
            compute_composite_column_name("emails", "primaryEmail"),
            "emailsPrimaryEmail"
        );
        assert_eq!(
            compute_composite_column_name("emails", "additionalEmails"),
            "emailsAdditionalEmails"
        );
        assert_eq!(
            compute_composite_column_name("phones", "primaryPhoneNumber"),
            "phonesPrimaryPhoneNumber"
        );
        assert_eq!(
            compute_composite_column_name("linkedinLink", "primaryLinkUrl"),
            "linkedinLinkPrimaryLinkUrl"
        );
        assert_eq!(
            compute_composite_column_name("address", "addressStreet1"),
            "addressAddressStreet1"
        );
        assert_eq!(
            compute_composite_column_name("amount", "amountMicros"),
            "amountAmountMicros"
        );
        assert_eq!(
            compute_composite_column_name("bodyV2", "blocknote"),
            "bodyV2Blocknote"
        );
        assert_eq!(
            compute_composite_column_name("createdBy", "workspaceMemberId"),
            "createdByWorkspaceMemberId"
        );
    }

    #[test]
    fn join_column_names_append_id() {
        assert_eq!(compute_relation_join_column_name("company"), "companyId");
        assert_eq!(
            compute_relation_join_column_name("accountOwner"),
            "accountOwnerId"
        );
    }

    #[test]
    fn server_only_composite_column_names_use_an_underscore() {
        assert_eq!(
            compute_server_only_composite_column_name("bodyV2", "fileIds"),
            "bodyV2_fileIds"
        );
    }

    #[test]
    fn camel_case_matches_lodash_golden_vectors() {
        let vectors: [(&str, &str); 40] = [
            ("fooBar", "fooBar"),
            ("foo1Bar", "foo1Bar"),
            ("fooBAR", "fooBar"),
            ("Foo Bar", "fooBar"),
            ("--foo-bar--", "fooBar"),
            ("__FOO_BAR__", "fooBar"),
            ("XMLHttpRequest", "xmlHttpRequest"),
            ("new_app", "newApp"),
            ("app_2", "app2"),
            ("a_b_c", "aBC"),
            ("test", "test"),
            ("helloWorld", "helloWorld"),
            ("HELLO", "hello"),
            ("hello", "hello"),
            ("h1", "h1"),
            ("1hello", "1Hello"),
            ("XMLHttpRequest2", "xmlHttpRequest2"),
            ("jobTitle", "jobTitle"),
            ("primaryPhoneNumber", "primaryPhoneNumber"),
            ("addressStreet1", "addressStreet1"),
            ("bodyV2", "bodyV2"),
            ("firstName", "firstName"),
            ("ID", "id"),
            ("someField", "someField"),
            ("some_field", "someField"),
            ("_leading", "leading"),
            ("trailing_", "trailing"),
            ("mi_ddle", "miDdle"),
            ("double__under", "doubleUnder"),
            ("UPPER_CASE", "upperCase"),
            ("upperCASE", "upperCase"),
            ("camelCase", "camelCase"),
            ("ABCdef", "abCdef"),
            ("abcDEF", "abcDef"),
            ("abc1DEF2", "abc1Def2"),
            ("field2FA", "field2Fa"),
            ("user2ID", "user2Id"),
            ("test123", "test123"),
            ("TEST123", "test123"),
            ("123test", "123Test"),
        ];

        for (input, expected) in vectors {
            assert_eq!(camel_case(input), expected, "input {input:?}");
        }
    }

    #[test]
    fn camel_case_deburrs_latin_letters_like_lodash() {
        assert_eq!(camel_case("hélloWörld"), "helloWorld");
        assert_eq!(camel_case("ÀCCRUEL"), "accruel");
        assert_eq!(camel_case("naïveCafé"), "naiveCafe");
        assert_eq!(camel_case(" straße "), "strasse");
        assert_eq!(camel_case("ABCd"), "abCd");
        assert_eq!(camel_case("AB Cd"), "abCd");
        assert_eq!(camel_case("e2eTest"), "e2ETest");
        assert_eq!(camel_case("v2"), "v2");
        assert_eq!(camel_case("A1B2c3"), "a1B2C3");
        assert_eq!(camel_case("a-b-c"), "aBC");
        assert_eq!(camel_case("RocketModel"), "rocketModel");
        assert_eq!(camel_case("Æther"), "aether");
        assert_eq!(camel_case("œuvre"), "oeuvre");
    }

    #[test]
    fn camel_case_splits_upper_runs_before_digits_like_lodash() {
        assert_eq!(camel_case("AB1"), "ab1");
        assert_eq!(camel_case("ABc"), "aBc");
        assert_eq!(camel_case("ab1"), "ab1");
        assert_eq!(camel_case("Ab1"), "ab1");
        assert_eq!(camel_case("ABCDx"), "abcDx");
        assert_eq!(camel_case("AB"), "ab");
        assert_eq!(camel_case("a1B2"), "a1B2");
        assert_eq!(camel_case("ABC"), "abc");
        assert_eq!(camel_case("ABC1"), "abc1");
        assert_eq!(camel_case("aB1"), "aB1");
        assert_eq!(camel_case("TestAB1"), "testAb1");
        assert_eq!(camel_case("x9Y"), "x9Y");
    }

    #[test]
    fn pascal_case_capitalizes_the_camel_cased_input() {
        assert_eq!(pascal_case("firstName"), "FirstName");
        assert_eq!(pascal_case("addressStreet1"), "AddressStreet1");
        assert_eq!(pascal_case("blocknote"), "Blocknote");
        assert_eq!(pascal_case("primaryPhoneNumber"), "PrimaryPhoneNumber");
        assert_eq!(pascal_case(""), "");
    }

    #[test]
    fn slugify_matches_the_transliteration_package_golden_vectors() {
        let vectors: [(&str, &str); 16] = [
            ("Rocket Model", "rocket_model"),
            ("My Model", "my_model"),
            ("New App", "new_app"),
            ("opportunity", "opportunity"),
            ("3D Printer", "3d_printer"),
            ("R&D Projects", "r_d_projects"),
            ("foo__bar", "foo_bar"),
            ("  spaced  out  ", "spaced_out"),
            ("héllo wörld", "hello_world"),
            ("naïve café", "naive_cafe"),
            ("Ünicode", "unicode"),
            ("ÀCCRUEL", "accruel"),
            ("a-b-c", "a_b_c"),
            ("name.with.dots", "name_with_dots"),
            ("100% Done", "100_done"),
            ("FOO BAR", "foo_bar"),
        ];

        for (input, expected) in vectors {
            assert_eq!(slugify(input), expected, "input {input:?}");
        }
    }

    #[test]
    fn metadata_name_from_label_computes_camel_cased_names() {
        assert_eq!(
            compute_metadata_name_from_label("Rocket Model", true).unwrap(),
            "rocketModel"
        );
        assert_eq!(
            compute_metadata_name_from_label("My Model", true).unwrap(),
            "myModel"
        );
        assert_eq!(
            compute_metadata_name_from_label("3D Printer", true).unwrap(),
            "n3DPrinter"
        );
        assert_eq!(compute_metadata_name_from_label("", true).unwrap(), "");
    }

    #[test]
    fn metadata_name_from_label_appends_custom_to_reserved_names() {
        assert_eq!(
            compute_metadata_name_from_label("User", true).unwrap(),
            "userCustom"
        );
        assert_eq!(
            compute_metadata_name_from_label("User", false).unwrap(),
            "user"
        );
    }

    #[test]
    fn metadata_name_from_label_prefixes_n_on_leading_digits() {
        assert_eq!(
            compute_metadata_name_from_label("100% Done", true).unwrap(),
            "n100Done"
        );
    }

    #[test]
    fn metadata_name_from_label_rejects_labels_without_usable_characters() {
        let error = compute_metadata_name_from_label("😀😀😀", true).unwrap_err();
        assert_eq!(error.message, "Invalid label: \"😀😀😀\"");
    }

    #[test]
    fn postgres_enum_name_strips_everything_outside_letters_digits_and_underscores() {
        assert_eq!(
            compute_postgres_enum_name("person", "status"),
            "person_status_enum"
        );
        assert_eq!(
            compute_postgres_enum_name("_person", "stage"),
            "_person_stage_enum"
        );
        assert_eq!(
            compute_postgres_enum_name("opportuni ty", "sta-ge"),
            "opportunity_stage_enum"
        );
    }

    #[test]
    fn remove_sql_ddl_injection_keeps_only_word_characters() {
        assert_eq!(remove_sql_ddl_injection("a-b.c;d"), "abcd");
        assert_eq!(remove_sql_ddl_injection("keep_1"), "keep_1");
    }

    #[test]
    fn object_name_validation_reports_every_upstream_error() {
        let errors = validate_object_metadata_names("person", "person");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "INVALID_OBJECT_INPUT");
        assert_eq!(
            errors[0].message,
            "The singular and plural names cannot be the same for an object"
        );

        let errors = validate_object_metadata_names("Person", "people");
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Name is not valid: it must start with lowercase letter and contain only alphanumeric letters"
        );

        let errors = validate_object_metadata_names("fooBAR", "fooBars");
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Name is not valid: it must be camelCase, e.g. \"fooBar\""
        );

        let long_name = "a".repeat(64);
        let errors = validate_object_metadata_names(&long_name, "people");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Name is too long");

        let errors = validate_object_metadata_names("", "people");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].message, "Name is too short");
        assert_eq!(
            errors[1].message,
            "Name is not valid: it must start with lowercase letter and contain only alphanumeric letters"
        );

        let errors = validate_object_metadata_names("user", "users");
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors[0].message,
            "This name is reserved. Use a different name or the system will add \"Custom\" suffix."
        );
        assert_eq!(
            errors[1].message,
            "This name is reserved. Use a different name or the system will add \"Custom\" suffix."
        );

        assert!(validate_object_metadata_names("company", "companies").is_empty());
        assert!(validate_object_metadata_names("workspaceMember", "workspaceMembers").is_empty());
    }

    #[test]
    fn field_name_validation_reports_the_upstream_errors() {
        let errors = validate_field_metadata_name("job", false);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "INVALID_FIELD_INPUT");
        assert_eq!(
            errors[0].message,
            "This name is reserved. Use a different name or the system will add \"Custom\" suffix."
        );

        assert!(validate_field_metadata_name("job", true).is_empty());

        let errors = validate_field_metadata_name("Company", false);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Name is not valid: it must start with lowercase letter and contain only alphanumeric letters"
        );

        assert!(validate_field_metadata_name(&"a".repeat(64), false)
            .iter()
            .any(|error| error.message == "Name is too long"));
    }

    #[test]
    fn duplicate_object_detection_cross_matches_singular_and_plural() {
        let existing = vec![
            (
                "20202020-1".to_string(),
                "person".to_string(),
                "people".to_string(),
            ),
            (
                "20202020-2".to_string(),
                "company".to_string(),
                "companies".to_string(),
            ),
        ];

        assert!(does_other_object_with_same_name_exists(
            "person", "persons", None, &existing
        ));
        assert!(does_other_object_with_same_name_exists(
            "personX", "people", None, &existing
        ));
        assert!(!does_other_object_with_same_name_exists(
            "rocket", "rockets", None, &existing
        ));
        assert!(!does_other_object_with_same_name_exists(
            "person",
            "persons",
            Some("20202020-1"),
            &existing
        ));
    }
}
