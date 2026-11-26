#![allow(dead_code)]
#![allow(unused_imports)]

pub mod enums;
pub mod field;
pub mod index;
pub mod maps;
pub mod object;
pub mod option;
pub mod rows;
pub mod search_field;
pub mod timestamp;

pub use crate::metadata::builders::UniversalEntityLike;
pub use enums::FieldMetadataType;
pub use enums::IndexType;
pub use enums::ObjectOpenRecordIn;
pub use enums::RelationType;
pub use field::FieldMinimalInformation;
pub use field::FlatFieldMetadata;
pub use field::UniversalFlatFieldMetadata;
pub use index::FlatIndexFieldMetadata;
pub use index::FlatIndexMetadata;
pub use index::IndexMinimalInformation;
pub use index::UniversalFlatIndexFieldMetadata;
pub use index::UniversalFlatIndexMetadata;
pub use maps::FlatEntityMaps;
pub use maps::FlatWorkspaceMaps;
pub use maps::FromToAllMaps;
pub use maps::FromToMaps;
pub use maps::UniversalFlatEntityMaps;
pub use maps::UniversalWorkspaceMaps;
pub use object::FlatObjectMetadata;
pub use object::ObjectMinimalInformation;
pub use object::UniversalFlatObjectMetadata;
pub use option::FieldMetadataOptionMinimalInformation;
pub use option::FlatFieldMetadataOption;
pub use option::UniversalFlatFieldMetadataOption;
pub use rows::assemble_workspace_maps;
pub use rows::load_workspace_flat_maps;
pub use search_field::FlatSearchFieldMetadata;
pub use search_field::SearchFieldMetadataMinimalInformation;
pub use search_field::UniversalFlatSearchFieldMetadata;

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono::Utc;
    use serde_json::json;

    use super::field::FlatFieldMetadata;
    use super::field::UniversalFlatFieldMetadata;
    use super::index::FlatIndexFieldMetadata;
    use super::index::FlatIndexMetadata;
    use super::index::UniversalFlatIndexFieldMetadata;
    use super::index::UniversalFlatIndexMetadata;
    use super::object::FlatObjectMetadata;
    use super::object::UniversalFlatObjectMetadata;
    use super::option::FlatFieldMetadataOption;
    use super::option::UniversalFlatFieldMetadataOption;
    use super::search_field::FlatSearchFieldMetadata;
    use crate::metadata::flat::enums::FieldMetadataType;
    use crate::metadata::flat::ObjectOpenRecordIn;

    fn sample_timestamp() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_flat_object() -> FlatObjectMetadata {
        FlatObjectMetadata {
            id: "550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
            universal_identifier: "20202020-e674-48e5-a542-72570eee7213".to_string(),
            application_id: "6f7d3e8a-1c2b-4a5d-9e8f-0a1b2c3d4e5f".parse().unwrap(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".parse().unwrap(),
            name_singular: "person".to_string(),
            name_plural: "people".to_string(),
            label_singular: "Person".to_string(),
            label_plural: "People".to_string(),
            is_searchable: true,
            open_record_in: Some(ObjectOpenRecordIn::UserChoice),
            target_table_name: Some("DEPRECATED".to_string()),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            application_universal_identifier: "6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264".to_string(),
            label_identifier_field_metadata_universal_identifier: Some(
                "20202020-4d99-46a1-9b8c-33e6c9b0d5c6".to_string(),
            ),
            ..FlatObjectMetadata::default()
        }
    }

    #[test]
    fn flat_object_metadata_serializes_upstream_shape() {
        let flat = sample_flat_object();
        let value = serde_json::to_value(&flat).unwrap();

        let expected_pairs: Vec<(&str, serde_json::Value)> = vec![
            ("id", json!("550e8400-e29b-41d4-a716-446655440000")),
            (
                "universalIdentifier",
                json!("20202020-e674-48e5-a542-72570eee7213"),
            ),
            (
                "applicationId",
                json!("6f7d3e8a-1c2b-4a5d-9e8f-0a1b2c3d4e5f"),
            ),
            ("workspaceId", json!("7c9e6679-7425-40de-944b-e07fc1f90ae7")),
            ("nameSingular", json!("person")),
            ("namePlural", json!("people")),
            ("labelSingular", json!("Person")),
            ("labelPlural", json!("People")),
            ("description", serde_json::Value::Null),
            ("isRemote", json!(false)),
            ("isActive", json!(false)),
            ("isSystem", json!(false)),
            ("isSearchable", json!(true)),
            ("isAuditLogged", json!(true)),
            ("isUIEditable", json!(true)),
            ("isUICreatable", json!(true)),
            ("disabledObjectPermissionActions", serde_json::Value::Null),
            ("openRecordIn", json!("USER_CHOICE")),
            ("isLabelSyncedWithName", json!(false)),
            ("overrides", serde_json::Value::Null),
            ("duplicateCriteria", serde_json::Value::Null),
            ("targetTableName", json!("DEPRECATED")),
            ("fieldIds", json!([])),
            ("indexMetadataIds", json!([])),
            ("searchFieldMetadataIds", json!([])),
            ("navigationMenuItemIds", json!([])),
            ("commandMenuItemIds", json!([])),
            ("objectPermissionIds", json!([])),
            ("fieldPermissionIds", json!([])),
            ("rowLevelPermissionRuleIds", json!([])),
            ("viewIds", json!([])),
            ("pageLayoutIds", json!([])),
            ("createdAt", json!("2024-01-01T00:00:00.000Z")),
            ("updatedAt", json!("2024-01-01T00:00:00.000Z")),
            (
                "applicationUniversalIdentifier",
                json!("6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264"),
            ),
            ("fieldUniversalIdentifiers", json!([])),
            ("objectPermissionUniversalIdentifiers", json!([])),
            ("fieldPermissionUniversalIdentifiers", json!([])),
            ("rowLevelPermissionRuleUniversalIdentifiers", json!([])),
            ("viewUniversalIdentifiers", json!([])),
            ("indexMetadataUniversalIdentifiers", json!([])),
            ("searchFieldMetadataUniversalIdentifiers", json!([])),
            ("pageLayoutUniversalIdentifiers", json!([])),
            ("navigationMenuItemUniversalIdentifiers", json!([])),
            ("commandMenuItemUniversalIdentifiers", json!([])),
            (
                "labelIdentifierFieldMetadataUniversalIdentifier",
                json!("20202020-4d99-46a1-9b8c-33e6c9b0d5c6"),
            ),
        ];

        let expected = serde_json::Value::Object(
            expected_pairs
                .into_iter()
                .map(|(key, entry)| (key.to_string(), entry))
                .collect(),
        );

        assert_eq!(value, expected);
    }

    #[test]
    fn flat_object_metadata_round_trips() {
        let flat = sample_flat_object();
        let serialized = serde_json::to_string(&flat).unwrap();
        let deserialized: FlatObjectMetadata = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, flat);
    }

    #[test]
    fn universal_object_metadata_drops_flat_only_keys() {
        let universal: UniversalFlatObjectMetadata = sample_flat_object().universal();

        let value = serde_json::to_value(&universal).unwrap();
        eprintln!(
            "PROBE universal keys: {:?}",
            value
                .as_object()
                .map(|object| object.keys().cloned().collect::<Vec<_>>())
        );

        assert!(value.get("id").is_none());
        assert!(value.get("workspaceId").is_none());
        assert!(value.get("applicationId").is_none());
        assert!(value.get("fieldIds").is_none());
        assert_eq!(
            value
                .get("universalIdentifier")
                .and_then(|raw| raw.as_str()),
            Some("20202020-e674-48e5-a542-72570eee7213")
        );
        assert_eq!(
            value
                .get("applicationUniversalIdentifier")
                .and_then(|raw| raw.as_str()),
            Some("6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264")
        );
    }

    fn sample_flat_field() -> FlatFieldMetadata {
        FlatFieldMetadata {
            id: "1d2c7f6e-5a4b-4c3d-8e2f-1a9b8c7d6e5f".parse().unwrap(),
            universal_identifier: "20202020-4d99-46a1-9b8c-33e6c9b0d5c6".to_string(),
            application_id: "6f7d3e8a-1c2b-4a5d-9e8f-0a1b2c3d4e5f".parse().unwrap(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".parse().unwrap(),
            object_metadata_id: "550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
            object_metadata_universal_identifier: "20202020-e674-48e5-a542-72570eee7213"
                .to_string(),
            field_type: FieldMetadataType::Text,
            name: "jobTitle".to_string(),
            label: "Job title".to_string(),
            is_active: true,
            is_nullable: true,
            is_audit_logged: true,
            universal_settings: Some(json!({ "displayedMaxRows": 4 })),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            ..FlatFieldMetadata::default()
        }
    }

    #[test]
    fn flat_field_metadata_serializes_upstream_shape() {
        let value = serde_json::to_value(sample_flat_field()).unwrap();

        assert_eq!(value.get("type").and_then(|raw| raw.as_str()), Some("TEXT"));
        assert_eq!(
            value.get("name").and_then(|raw| raw.as_str()),
            Some("jobTitle")
        );
        assert_eq!(
            value.get("universalSettings"),
            Some(&json!({ "displayedMaxRows": 4 }))
        );
        assert_eq!(
            value.get("createdAt").and_then(|raw| raw.as_str()),
            Some("2024-01-01T00:00:00.000Z")
        );
        assert_eq!(value.get("viewFieldIds"), Some(&json!([])));
    }

    #[test]
    fn universal_field_metadata_uses_universal_settings_key() {
        let universal: UniversalFlatFieldMetadata = sample_flat_field().universal();
        let value = serde_json::to_value(&universal).unwrap();

        assert!(value.get("id").is_none());
        assert!(value.get("objectMetadataId").is_none());
        assert!(value.get("settings").is_none());
        assert_eq!(
            value.get("universalSettings"),
            Some(&json!({ "displayedMaxRows": 4 }))
        );
        assert_eq!(
            value
                .get("objectMetadataUniversalIdentifier")
                .and_then(|raw| raw.as_str()),
            Some("20202020-e674-48e5-a542-72570eee7213")
        );
    }

    #[test]
    fn flat_field_metadata_round_trips() {
        let flat = sample_flat_field();
        let serialized = serde_json::to_string(&flat).unwrap();
        let deserialized: FlatFieldMetadata = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, flat);
    }

    #[test]
    fn flat_index_metadata_serializes_both_field_variants() {
        let index_field = FlatIndexFieldMetadata {
            id: "0f6c8a2b-3d4e-4f5a-9b8c-7d6e5f4a3b2c".parse().unwrap(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".parse().unwrap(),
            index_metadata_id: "2b7d9e0f-1a2b-4c3d-8e5f-6a7b8c9d0e1f".parse().unwrap(),
            field_metadata_id: "1d2c7f6e-5a4b-4c3d-8e2f-1a9b8c7d6e5f".parse().unwrap(),
            order: 0,
            sub_field_name: Some("primaryPhoneNumber".to_string()),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
        };

        let universal_index_field = UniversalFlatIndexFieldMetadata {
            order: 0,
            sub_field_name: Some("primaryPhoneNumber".to_string()),
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            index_metadata_universal_identifier: "20202020-cbf8-4df4-b19d-37ef05946c6d".to_string(),
            field_metadata_universal_identifier: "20202020-4d99-46a1-9b8c-33e6c9b0d5c6".to_string(),
        };

        let index = FlatIndexMetadata {
            id: "2b7d9e0f-1a2b-4c3d-8e5f-6a7b8c9d0e1f".parse().unwrap(),
            universal_identifier: "20202020-cbf8-4df4-b19d-37ef05946c6d".to_string(),
            application_id: "6f7d3e8a-1c2b-4a5d-9e8f-0a1b2c3d4e5f".parse().unwrap(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".parse().unwrap(),
            name: "IDX_PERSON_SEARCH".to_string(),
            object_metadata_id: "550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
            object_metadata_universal_identifier: "20202020-e674-48e5-a542-72570eee7213"
                .to_string(),
            index_where_clause: Some("\"deletedAt\" IS NULL".to_string()),
            flat_index_field_metadatas: vec![index_field],
            universal_flat_index_field_metadatas: vec![universal_index_field],
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            ..FlatIndexMetadata::default()
        };

        let value = serde_json::to_value(&index).unwrap();

        assert_eq!(
            value.get("indexType").and_then(|raw| raw.as_str()),
            Some("BTREE")
        );
        assert_eq!(
            value.get("indexWhereClause").and_then(|raw| raw.as_str()),
            Some("\"deletedAt\" IS NULL")
        );
        assert_eq!(
            value
                .get("flatIndexFieldMetadatas")
                .and_then(|raw| raw.as_array())
                .map(|array| array.len()),
            Some(1)
        );
        assert_eq!(
            value
                .get("universalFlatIndexFieldMetadatas")
                .and_then(|raw| raw.as_array())
                .map(|array| array.len()),
            Some(1)
        );

        let universal = UniversalFlatIndexMetadata {
            universal_identifier: index.universal_identifier.clone(),
            application_universal_identifier: "6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264".to_string(),
            name: index.name.clone(),
            object_metadata_universal_identifier: index
                .object_metadata_universal_identifier
                .clone(),
            is_unique: index.is_unique,
            index_where_clause: index.index_where_clause.clone(),
            index_type: index.index_type,
            is_system_side_effect: index.is_system_side_effect,
            created_at: index.created_at,
            updated_at: index.updated_at,
            universal_flat_index_field_metadatas: index
                .universal_flat_index_field_metadatas
                .clone(),
        };

        let universal_value = serde_json::to_value(&universal).unwrap();

        assert!(universal_value.get("id").is_none());
        assert!(universal_value.get("flatIndexFieldMetadatas").is_none());
        assert!(universal_value.get("objectMetadataId").is_none());
    }

    #[test]
    fn flat_option_and_search_field_round_trip() {
        let option = FlatFieldMetadataOption {
            id: "3c8e9f0a-1b2c-4d5e-8f9a-0b1c2d3e4f5a".parse().unwrap(),
            universal_identifier: "20202020-a45a-4be1-b2c3-d4e5f6a7b8c9".to_string(),
            application_id: "6f7d3e8a-1c2b-4a5d-9e8f-0a1b2c3d4e5f".parse().unwrap(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".parse().unwrap(),
            field_metadata_id: "1d2c7f6e-5a4b-4c3d-8e2f-1a9b8c7d6e5f".parse().unwrap(),
            field_metadata_universal_identifier: "20202020-4d99-46a1-9b8c-33e6c9b0d5c6".to_string(),
            value: "OPTION_1".to_string(),
            label: "Option 1".to_string(),
            color: Some("blue".to_string()),
            position: 0.0,
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            ..FlatFieldMetadataOption::default()
        };

        let option_value = serde_json::to_value(&option).unwrap();

        assert_eq!(
            option_value.get("isActive").and_then(|raw| raw.as_bool()),
            Some(true)
        );
        assert_eq!(
            option_value
                .get("fieldMetadataUniversalIdentifier")
                .and_then(|raw| raw.as_str()),
            Some("20202020-4d99-46a1-9b8c-33e6c9b0d5c6")
        );

        let option_deserialized: FlatFieldMetadataOption =
            serde_json::from_str(&serde_json::to_string(&option).unwrap()).unwrap();

        assert_eq!(option_deserialized, option);

        let universal_option: UniversalFlatFieldMetadataOption = option.clone().universal();
        let universal_value = serde_json::to_value(&universal_option).unwrap();

        assert!(universal_value.get("id").is_none());
        assert!(universal_value.get("fieldMetadataId").is_none());

        let search_field = FlatSearchFieldMetadata {
            id: "4d9f0a1b-2c3d-4e5f-8a9b-0c1d2e3f4a5b".parse().unwrap(),
            universal_identifier: "20202020-b56b-4cf2-c3d4-e5f6a7b8c9d0".to_string(),
            application_id: "6f7d3e8a-1c2b-4a5d-9e8f-0a1b2c3d4e5f".parse().unwrap(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".parse().unwrap(),
            object_metadata_id: "550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
            object_metadata_universal_identifier: "20202020-e674-48e5-a542-72570eee7213"
                .to_string(),
            field_metadata_id: "1d2c7f6e-5a4b-4c3d-8e2f-1a9b8c7d6e5f".parse().unwrap(),
            field_metadata_universal_identifier: "20202020-4d99-46a1-9b8c-33e6c9b0d5c6".to_string(),
            ts_vector_field_metadata_id: "5eaf1b2c-3d4e-4f5a-9b0c-1d2e3f4a5b6c".parse().unwrap(),
            ts_vector_field_metadata_universal_identifier: "20202020-c67c-4d03-d4e5-f6a7b8c9d0e1"
                .to_string(),
            position: 0.0,
            created_at: sample_timestamp(),
            updated_at: sample_timestamp(),
            ..FlatSearchFieldMetadata::default()
        };

        let search_field_value = serde_json::to_value(&search_field).unwrap();

        assert_eq!(
            search_field_value
                .get("tsVectorFieldMetadataUniversalIdentifier")
                .and_then(|raw| raw.as_str()),
            Some("20202020-c67c-4d03-d4e5-f6a7b8c9d0e1")
        );

        let search_field_deserialized: FlatSearchFieldMetadata =
            serde_json::from_str(&serde_json::to_string(&search_field).unwrap()).unwrap();

        assert_eq!(search_field_deserialized, search_field);
    }
}
