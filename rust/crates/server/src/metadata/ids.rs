#![allow(dead_code)]

use sha1::Digest;
use sha2::Sha256;

use super::names::compute_object_target_table;

pub const INDEX_NAME_PREFIX: &str = "IDX_";
pub const UNIQUE_INDEX_NAME_PREFIX: &str = "IDX_UNIQUE_";
pub const FOREIGN_KEY_NAME_PREFIX: &str = "FK_";
pub const HASHED_NAME_LENGTH: usize = 27;

pub fn generate_deterministic_index_name(
    table_name: &str,
    ordered_index_column_names: &[&str],
    is_unique: bool,
    index_where_clause: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();

    hasher.update(table_name.as_bytes());

    for column_name in ordered_index_column_names {
        hasher.update(column_name.as_bytes());
    }

    if let Some(where_clause) = index_where_clause {
        hasher.update(where_clause.as_bytes());
    }

    let digest = hex::encode(hasher.finalize());
    let prefix = if is_unique {
        UNIQUE_INDEX_NAME_PREFIX
    } else {
        INDEX_NAME_PREFIX
    };

    format!("{prefix}{}", &digest[..HASHED_NAME_LENGTH])
}

pub fn generate_object_deterministic_index_name(
    name_singular: &str,
    application_universal_identifier: &str,
    ordered_index_column_names: &[&str],
    is_unique: bool,
    index_where_clause: Option<&str>,
) -> String {
    let table_name = compute_object_target_table(name_singular, application_universal_identifier);

    generate_deterministic_index_name(
        &table_name,
        ordered_index_column_names,
        is_unique,
        index_where_clause,
    )
}

pub fn foreign_key_name(table_name: &str, column_name: &str) -> String {
    let mut hasher = sha1::Sha1::new();

    hasher.update(table_name.as_bytes());
    hasher.update(b"_");
    hasher.update(column_name.as_bytes());

    let digest = hex::encode(hasher.finalize());

    format!("{FOREIGN_KEY_NAME_PREFIX}{}", &digest[..HASHED_NAME_LENGTH])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_name_hashes_table_and_columns_in_order() {
        assert_eq!(
            generate_deterministic_index_name("company", &["accountOwnerId"], false, None),
            "IDX_f719a95179070eac397ba18dc70"
        );
        assert_ne!(
            generate_deterministic_index_name("company", &["accountOwnerId"], false, None),
            generate_deterministic_index_name("person", &["accountOwnerId"], false, None)
        );
    }

    #[test]
    fn index_name_golden_vectors_from_sha256() {
        assert_eq!(
            generate_deterministic_index_name(
                "person",
                &["emailsPrimaryEmail"],
                true,
                Some("\"deletedAt\" IS NULL")
            ),
            "IDX_UNIQUE_f68c4b0b86079af3bea4286d92e"
        );
        assert_eq!(
            generate_deterministic_index_name(
                "person",
                &["emailsPrimaryEmail"],
                false,
                Some("\"deletedAt\" IS NULL")
            ),
            "IDX_f68c4b0b86079af3bea4286d92e"
        );
        assert_eq!(
            generate_deterministic_index_name("company", &["accountOwnerId"], false, None),
            "IDX_f719a95179070eac397ba18dc70"
        );
        assert_eq!(
            generate_deterministic_index_name("person", &["searchVector"], false, None),
            "IDX_bbd7aec1976fc684a0a5e4816c9"
        );
        assert_eq!(
            generate_deterministic_index_name("_rocket", &["id"], false, None),
            "IDX_59c5b23cc3d23715f829ef969c7"
        );
    }

    #[test]
    fn index_where_clause_participates_in_the_hash() {
        let without_where = generate_deterministic_index_name("person", &["jobTitle"], false, None);
        let with_where = generate_deterministic_index_name(
            "person",
            &["jobTitle"],
            false,
            Some("\"deletedAt\" IS NULL"),
        );

        assert_ne!(without_where, with_where);
        assert_eq!(without_where, "IDX_b104498a122b82fc830e6e27a0c");
        assert_eq!(with_where, "IDX_017d1e29e1044ba3e5eb355af3b");
    }

    #[test]
    fn object_level_index_name_uses_the_custom_table_prefix() {
        assert_eq!(
            generate_object_deterministic_index_name(
                "person",
                super::super::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER,
                &["searchVector"],
                false,
                None
            ),
            "IDX_bbd7aec1976fc684a0a5e4816c9"
        );
        assert_eq!(
            generate_object_deterministic_index_name("rocket", "custom-app", &["id"], false, None),
            "IDX_59c5b23cc3d23715f829ef969c7"
        );
    }

    #[test]
    fn foreign_key_name_matches_the_typeorm_default_naming_strategy() {
        assert_eq!(
            foreign_key_name("workspace", "workspaceCustomApplicationId"),
            "FK_3b1acb13a5dac9956d1a4b32755"
        );
        assert_eq!(
            foreign_key_name("pageLayoutTab", "pageLayoutId"),
            "FK_0177b1574efe6e6f24651977340"
        );
        assert_eq!(
            foreign_key_name(
                "applicationRegistrationVersion",
                "applicationRegistrationId"
            ),
            "FK_02169996cebff7ce601ef1a8856"
        );
        assert_eq!(
            foreign_key_name("navigationMenuItem", "workspaceId"),
            "FK_03c63a0b00ddc3ade21ed0b1a80"
        );
        assert_eq!(
            foreign_key_name("viewField", "relationTargetFieldMetadataId"),
            "FK_04182cf3303e503661915eb27ee"
        );
    }

    #[test]
    fn foreign_key_name_hashes_only_table_and_column() {
        assert_eq!(
            foreign_key_name("person", "companyId"),
            "FK_ee066ddacfce46c9a7cb90edd1a"
        );
        assert_eq!(
            foreign_key_name("noteTarget", "noteId"),
            "FK_52050fd9cf6b7ba9982ef9d867a"
        );
    }
}
