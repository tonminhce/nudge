#![allow(dead_code)]

use crate::metadata::names::{compute_column_name, compute_composite_column_name};

use crate::workspace_migrations::column_defs::composite_types::{
    composite_properties, is_composite_field_metadata_type,
};
use crate::workspace_migrations::column_defs::{ColumnDefsError, FieldMetadataType};
use crate::workspace_migrations::ddl::escape::escape_identifier;

pub const SEARCHABLE_FIELD_TYPES: [FieldMetadataType; 8] = [
    FieldMetadataType::Text,
    FieldMetadataType::FullName,
    FieldMetadataType::Emails,
    FieldMetadataType::Address,
    FieldMetadataType::Links,
    FieldMetadataType::Phones,
    FieldMetadataType::RichText,
    FieldMetadataType::Uuid,
];

pub fn is_searchable_field_type(field_type: FieldMetadataType) -> bool {
    SEARCHABLE_FIELD_TYPES.contains(&field_type)
}

pub fn is_searchable_subfield(
    composite_field_type: FieldMetadataType,
    sub_field_type: FieldMetadataType,
    sub_field_name: &str,
) -> bool {
    if sub_field_type != FieldMetadataType::Text {
        return false;
    }

    match composite_field_type {
        FieldMetadataType::RichText => sub_field_name == "markdown",
        FieldMetadataType::Phones => {
            sub_field_name == "primaryPhoneNumber" || sub_field_name == "primaryPhoneCallingCode"
        }
        _ => true,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldTypeAndName {
    pub name: String,
    pub field_type: FieldMetadataType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchVectorTargetField {
    pub name: String,
    pub field_type: FieldMetadataType,
    pub position: i64,
    pub sort_key: String,
}

pub fn compute_search_vector_as_expression(
    target_searchable_fields: &[SearchVectorTargetField],
) -> Result<String, ColumnDefsError> {
    let mut ordered: Vec<&SearchVectorTargetField> = target_searchable_fields.iter().collect();

    ordered.sort_by(|a, b| match a.position.cmp(&b.position) {
        std::cmp::Ordering::Equal => a.sort_key.cmp(&b.sort_key),
        other => other,
    });

    let searchable_fields: Vec<FieldTypeAndName> = ordered
        .into_iter()
        .filter(|target_field| is_searchable_field_type(target_field.field_type))
        .map(|target_field| FieldTypeAndName {
            name: target_field.name.clone(),
            field_type: target_field.field_type,
        })
        .collect();

    get_ts_vector_column_expression_from_fields(&searchable_fields)
}

pub fn get_ts_vector_column_expression_from_fields(
    fields: &[FieldTypeAndName],
) -> Result<String, ColumnDefsError> {
    let mut column_expressions: Vec<String> = Vec::new();

    for field in fields {
        column_expressions.extend(get_column_expressions_from_field(field)?);
    }

    let concatenated_expression = if column_expressions.is_empty() {
        "NULL".to_string()
    } else {
        column_expressions.join(" || ' ' || ")
    };

    Ok(format!("to_tsvector('simple', {concatenated_expression})"))
}

fn quoted_column(column_name: &str) -> Result<String, ColumnDefsError> {
    escape_identifier(column_name).map_err(|error| ColumnDefsError::IdentifierEscape {
        message: error.to_string(),
    })
}

fn get_column_expressions_from_field(
    field: &FieldTypeAndName,
) -> Result<Vec<String>, ColumnDefsError> {
    if is_composite_field_metadata_type(field.field_type) {
        let properties = composite_properties(field.field_type);

        let mut expressions: Vec<String> = properties
            .iter()
            .filter(|property| {
                is_searchable_subfield(field.field_type, property.field_type, property.name)
            })
            .map(|property| {
                let column_name = compute_composite_column_name(&field.name, property.name);
                get_column_expression(&column_name, field.field_type)
            })
            .collect::<Result<Vec<_>, _>>()?;

        if field.field_type == FieldMetadataType::Phones {
            let phone_number_column = quoted_column(&format!("{}PrimaryPhoneNumber", field.name))?;
            let calling_code_column =
                quoted_column(&format!("{}PrimaryPhoneCallingCode", field.name))?;
            let additional_phones_column =
                quoted_column(&format!("{}AdditionalPhones", field.name))?;

            expressions.push(format!(
                "COALESCE({calling_code_column} || {phone_number_column}, '')"
            ));
            expressions.push(format!(
                "COALESCE(REPLACE({calling_code_column}, '+', '') || {phone_number_column}, '')"
            ));
            expressions.push(format!("COALESCE('0' || {phone_number_column}, '')"));
            expressions.push(format!(
                "COALESCE(TRANSLATE(regexp_replace({additional_phones_column}::text, '\"(number|countryCode|callingCode)\"\\s*:\\s*', '', 'g'), '[]{{}}\",:', '        '), '')"
            ));

            return Ok(expressions);
        }

        if field.field_type == FieldMetadataType::Links {
            let secondary_links_column = quoted_column(&format!("{}SecondaryLinks", field.name))?;

            expressions.push(format!(
                "COALESCE(public.unaccent_immutable(TRANSLATE(regexp_replace({secondary_links_column}::text, '\"(label|url)\"\\s*:\\s*', '', 'g'), '[]{{}}\",:', '        ')), '')"
            ));

            return Ok(expressions);
        }

        if field.field_type == FieldMetadataType::Emails {
            let additional_emails_column =
                quoted_column(&format!("{}AdditionalEmails", field.name))?;

            expressions.push(format!(
                "COALESCE(public.unaccent_immutable(TRANSLATE({additional_emails_column}::text, '[]\",', '    ')), '') || ' ' || COALESCE(public.unaccent_immutable(TRANSLATE(REPLACE({additional_emails_column}::text, '@', ' '), '[]\",', '    ')), '')"
            ));

            return Ok(expressions);
        }

        return Ok(expressions);
    }

    let column_name = compute_column_name(&field.name);

    Ok(vec![get_column_expression(&column_name, field.field_type)?])
}

fn get_column_expression(
    column_name: &str,
    field_type: FieldMetadataType,
) -> Result<String, ColumnDefsError> {
    let quoted_column_name = quoted_column(column_name)?;

    match field_type {
        FieldMetadataType::Emails => Ok(format!(
            "\n      COALESCE(public.unaccent_immutable({quoted_column_name}), '') || ' ' ||\n      COALESCE(public.unaccent_immutable(SPLIT_PART({quoted_column_name}, '@', 2)), '')"
        )),
        FieldMetadataType::Phones => Ok(format!("COALESCE({quoted_column_name}, '')")),
        FieldMetadataType::Uuid => Ok(format!("COALESCE({quoted_column_name}::text, '')")),
        _ => Ok(format!(
            "COALESCE(public.unaccent_immutable({quoted_column_name}), '')"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_field(name: &str) -> FieldTypeAndName {
        FieldTypeAndName {
            name: name.to_string(),
            field_type: FieldMetadataType::Text,
        }
    }

    fn target(name: &str, field_type: FieldMetadataType, position: i64) -> SearchVectorTargetField {
        SearchVectorTargetField {
            name: name.to_string(),
            field_type,
            position,
            sort_key: format!("sort-{position}"),
        }
    }

    #[test]
    fn simple_text_field_wraps_the_column_in_unaccent() {
        let expression =
            get_ts_vector_column_expression_from_fields(&[text_field("name")]).unwrap();

        assert_eq!(
            expression,
            "to_tsvector('simple', COALESCE(public.unaccent_immutable(\"name\"), ''))"
        );
    }

    #[test]
    fn body_field_expression_is_byte_identical_to_the_upstream_spec() {
        let expression =
            get_ts_vector_column_expression_from_fields(&[text_field("body")]).unwrap();

        assert_eq!(
            expression,
            "to_tsvector('simple', COALESCE(public.unaccent_immutable(\"body\"), ''))"
        );
    }

    #[test]
    fn rich_text_fields_index_only_the_markdown_subfield() {
        let fields = vec![FieldTypeAndName {
            name: "bodyV2".to_string(),
            field_type: FieldMetadataType::RichText,
        }];

        assert_eq!(
            get_ts_vector_column_expression_from_fields(&fields).unwrap(),
            "to_tsvector('simple', COALESCE(public.unaccent_immutable(\"bodyV2Markdown\"), ''))"
        );
    }

    #[test]
    fn empty_field_lists_produce_a_null_expression() {
        assert_eq!(
            get_ts_vector_column_expression_from_fields(&[]).unwrap(),
            "to_tsvector('simple', NULL)"
        );
    }

    #[test]
    fn phone_fields_skip_unaccent_and_add_international_and_trunk_formats() {
        let fields = vec![FieldTypeAndName {
            name: "phones".to_string(),
            field_type: FieldMetadataType::Phones,
        }];

        let expression = get_ts_vector_column_expression_from_fields(&fields).unwrap();

        assert!(expression.contains("COALESCE(\"phonesPrimaryPhoneNumber\", '')"));
        assert!(expression.contains("COALESCE(\"phonesPrimaryPhoneCallingCode\", '')"));
        assert!(!expression.contains("unaccent_immutable"));
        assert!(expression.contains(
            "COALESCE(\"phonesPrimaryPhoneCallingCode\" || \"phonesPrimaryPhoneNumber\", '')"
        ));
        assert!(expression.contains(
            "COALESCE(REPLACE(\"phonesPrimaryPhoneCallingCode\", '+', '') || \"phonesPrimaryPhoneNumber\", '')"
        ));
        assert!(expression.contains("COALESCE('0' || \"phonesPrimaryPhoneNumber\", '')"));
        assert!(expression.contains(
            "COALESCE(TRANSLATE(regexp_replace(\"phonesAdditionalPhones\"::text, '\"(number|countryCode|callingCode)\"\\s*:\\s*', '', 'g'), '[]{}\",:', '        '), '')"
        ));
    }

    #[test]
    fn links_fields_append_the_secondary_links_expression() {
        let fields = vec![FieldTypeAndName {
            name: "domainName".to_string(),
            field_type: FieldMetadataType::Links,
        }];

        let expression = get_ts_vector_column_expression_from_fields(&fields).unwrap();

        assert!(expression.contains("domainNamePrimaryLinkLabel"));
        assert!(expression.contains("domainNamePrimaryLinkUrl"));
        assert!(expression.contains("domainNameSecondaryLinks"));
        assert!(expression.contains(
            "COALESCE(public.unaccent_immutable(TRANSLATE(regexp_replace(\"domainNameSecondaryLinks\"::text, '\"(label|url)\"\\s*:\\s*', '', 'g'), '[]{}\",:', '        ')), '')"
        ));
    }

    #[test]
    fn emails_fields_add_the_domain_split_and_additional_emails_expressions() {
        let fields = vec![FieldTypeAndName {
            name: "emails".to_string(),
            field_type: FieldMetadataType::Emails,
        }];

        let expression = get_ts_vector_column_expression_from_fields(&fields).unwrap();

        assert!(
            expression.contains("COALESCE(public.unaccent_immutable(\"emailsPrimaryEmail\"), '')")
        );
        assert!(expression.contains(
            "COALESCE(public.unaccent_immutable(SPLIT_PART(\"emailsPrimaryEmail\", '@', 2)), '')"
        ));
        assert!(expression.contains(
            "COALESCE(public.unaccent_immutable(TRANSLATE(\"emailsAdditionalEmails\"::text, '[]\",', '    ')), '')"
        ));
        assert!(expression.contains(
            "COALESCE(public.unaccent_immutable(TRANSLATE(REPLACE(\"emailsAdditionalEmails\"::text, '@', ' '), '[]\",', '    ')), '')"
        ));
    }

    #[test]
    fn uuid_columns_cast_to_text() {
        let fields = vec![FieldTypeAndName {
            name: "linkedRecordId".to_string(),
            field_type: FieldMetadataType::Uuid,
        }];

        assert_eq!(
            get_ts_vector_column_expression_from_fields(&fields).unwrap(),
            "to_tsvector('simple', COALESCE(\"linkedRecordId\"::text, ''))"
        );
    }

    #[test]
    fn non_searchable_field_types_are_filtered_before_building_the_expression() {
        let expression = compute_search_vector_as_expression(&[
            target("amount", FieldMetadataType::Number, 0),
            target("jobTitle", FieldMetadataType::Text, 1),
        ])
        .unwrap();

        assert_eq!(
            expression,
            "to_tsvector('simple', COALESCE(public.unaccent_immutable(\"jobTitle\"), ''))"
        );
    }

    #[test]
    fn fields_are_ordered_by_position_then_sort_key() {
        let expression = compute_search_vector_as_expression(&[
            target("jobTitle", FieldMetadataType::Text, 3),
            target("name", FieldMetadataType::FullName, 0),
        ])
        .unwrap();

        assert_eq!(
            expression,
            "to_tsvector('simple', COALESCE(public.unaccent_immutable(\"nameFirstName\"), '') || ' ' || COALESCE(public.unaccent_immutable(\"nameLastName\"), '') || ' ' || COALESCE(public.unaccent_immutable(\"jobTitle\"), ''))"
        );
    }

    #[test]
    fn is_searchable_subfield_follows_the_upstream_rules() {
        assert!(is_searchable_subfield(
            FieldMetadataType::FullName,
            FieldMetadataType::Text,
            "firstName"
        ));
        assert!(is_searchable_subfield(
            FieldMetadataType::RichText,
            FieldMetadataType::Text,
            "markdown"
        ));
        assert!(!is_searchable_subfield(
            FieldMetadataType::RichText,
            FieldMetadataType::Text,
            "blocknote"
        ));
        assert!(!is_searchable_subfield(
            FieldMetadataType::RichText,
            FieldMetadataType::Text,
            "tiptap"
        ));
        assert!(is_searchable_subfield(
            FieldMetadataType::Phones,
            FieldMetadataType::Text,
            "primaryPhoneNumber"
        ));
        assert!(is_searchable_subfield(
            FieldMetadataType::Phones,
            FieldMetadataType::Text,
            "primaryPhoneCallingCode"
        ));
        assert!(!is_searchable_subfield(
            FieldMetadataType::Phones,
            FieldMetadataType::Text,
            "primaryPhoneCountryCode"
        ));
        assert!(!is_searchable_subfield(
            FieldMetadataType::Phones,
            FieldMetadataType::RawJson,
            "additionalPhones"
        ));
        assert!(!is_searchable_subfield(
            FieldMetadataType::Emails,
            FieldMetadataType::RawJson,
            "additionalEmails"
        ));
        assert!(is_searchable_subfield(
            FieldMetadataType::Links,
            FieldMetadataType::Text,
            "primaryLinkUrl"
        ));
    }

    #[test]
    fn person_expression_is_byte_equal_to_the_reference_snapshot() {
        let expected = concat!(
            "to_tsvector('simple', ",
            "COALESCE(public.unaccent_immutable(\"nameFirstName\"), '') || ' ' || ",
            "COALESCE(public.unaccent_immutable(\"nameLastName\"), '') || ' ' || ",
            "\n      COALESCE(public.unaccent_immutable(\"emailsPrimaryEmail\"), '') || ' ' ||\n      ",
            "COALESCE(public.unaccent_immutable(SPLIT_PART(\"emailsPrimaryEmail\", '@', 2)), '') || ' ' || ",
            "COALESCE(public.unaccent_immutable(TRANSLATE(\"emailsAdditionalEmails\"::text, '[]\",', '    ')), '') || ' ' || ",
            "COALESCE(public.unaccent_immutable(TRANSLATE(REPLACE(\"emailsAdditionalEmails\"::text, '@', ' '), '[]\",', '    ')), '') || ' ' || ",
            "COALESCE(\"phonesPrimaryPhoneNumber\", '') || ' ' || ",
            "COALESCE(\"phonesPrimaryPhoneCallingCode\", '') || ' ' || ",
            "COALESCE(\"phonesPrimaryPhoneCallingCode\" || \"phonesPrimaryPhoneNumber\", '') || ' ' || ",
            "COALESCE(REPLACE(\"phonesPrimaryPhoneCallingCode\", '+', '') || \"phonesPrimaryPhoneNumber\", '') || ' ' || ",
            "COALESCE('0' || \"phonesPrimaryPhoneNumber\", '') || ' ' || ",
            "COALESCE(TRANSLATE(regexp_replace(\"phonesAdditionalPhones\"::text, '\"(number|countryCode|callingCode)\"\\s*:\\s*', '', 'g'), '[]{}\",:', '        '), '') || ' ' || ",
            "COALESCE(public.unaccent_immutable(\"jobTitle\"), ''))"
        );

        let expression = compute_search_vector_as_expression(&[
            target("name", FieldMetadataType::FullName, 0),
            target("emails", FieldMetadataType::Emails, 1),
            target("phones", FieldMetadataType::Phones, 2),
            target("jobTitle", FieldMetadataType::Text, 3),
        ])
        .unwrap();

        assert_eq!(expression, expected);
    }
}
