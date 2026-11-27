use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use serde_json::Value;
use uuid::Uuid;

use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatEntityMaps;
use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatObjectMetadata;
use crate::metadata::flat::RelationType;
use crate::metadata::names::compute_composite_column_name;
use crate::metadata::names::compute_object_target_table;
use crate::metadata::names::compute_relation_join_column_name;
use crate::workspace_migrations::column_defs::composite_types::composite_properties;
use crate::workspace_migrations::column_defs::composite_types::is_composite_field_metadata_type;
use crate::workspace_migrations::runner::ddl_field_type;

use super::RepositoryError;

pub const DEFAULT_TABLE_SHAPE_CACHE_CAPACITY: usize = 4096;

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnShape {
    pub column_name: String,
    pub field_metadata_id: Uuid,
    pub field_name: String,
    pub field_metadata_type: FieldMetadataType,
    pub composite_parent_field_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationShape {
    pub field_name: String,
    pub field_metadata_id: Uuid,
    pub relation_type: RelationType,
    pub target_object_metadata_id: Option<Uuid>,
    pub target_field_metadata_id: Option<Uuid>,
    pub join_column_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableShape {
    pub object_metadata_id: Uuid,
    pub name_singular: String,
    pub schema_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnShape>,
    pub column_names: Vec<String>,
    pub column_shape_by_column_name: HashMap<String, ColumnShape>,
    pub relation_shape_by_field_name: HashMap<String, RelationShape>,
    pub has_deleted_at_column: bool,
}

impl TableShape {
    pub fn column(&self, column_name: &str) -> Option<&ColumnShape> {
        self.column_shape_by_column_name.get(column_name)
    }

    pub fn relation(&self, field_name: &str) -> Option<&RelationShape> {
        self.relation_shape_by_field_name.get(field_name)
    }
}

fn is_morph_or_relation_field(field: &FlatFieldMetadata) -> bool {
    matches!(
        field.field_type,
        FieldMetadataType::Relation | FieldMetadataType::MorphRelation
    )
}

fn is_morph_relation_head(field: &FlatFieldMetadata) -> bool {
    field.field_type == FieldMetadataType::MorphRelation
        && field.morph_id.map(|id| id.to_string()) == Some(field.universal_identifier.clone())
}

fn relation_type_from_settings(settings: &Option<Value>) -> Option<RelationType> {
    settings
        .as_ref()?
        .get("relationType")?
        .as_str()
        .and_then(RelationType::parse)
}

fn target_table_name(object: &FlatObjectMetadata) -> String {
    match &object.target_table_name {
        Some(name) if name != "DEPRECATED" => name.clone(),
        _ => compute_object_target_table(
            &object.name_singular,
            &object.application_universal_identifier,
        ),
    }
}

pub fn build_workspace_table_shape(
    workspace_id: &Uuid,
    flat_object_metadata: &FlatObjectMetadata,
    flat_field_metadata_maps: &FlatEntityMaps<FlatFieldMetadata>,
) -> TableShape {
    let mut column_shape_by_column_name: HashMap<String, ColumnShape> = HashMap::new();
    let mut column_names: Vec<String> = Vec::new();
    let mut relation_shape_by_field_name: HashMap<String, RelationShape> = HashMap::new();

    let mut push_column = |shape: ColumnShape| {
        if !column_shape_by_column_name.contains_key(&shape.column_name) {
            column_names.push(shape.column_name.clone());
        }
        column_shape_by_column_name.insert(shape.column_name.clone(), shape);
    };

    for field in object_fields(flat_object_metadata, flat_field_metadata_maps) {
        if is_morph_relation_head(field) {
            continue;
        }

        if is_morph_or_relation_field(field) {
            let Some(relation_type) = relation_type_from_settings(&field.settings) else {
                continue;
            };

            let is_owning_side = relation_type == RelationType::ManyToOne;
            let join_column_name =
                is_owning_side.then(|| compute_relation_join_column_name(&field.name));

            relation_shape_by_field_name.insert(
                field.name.clone(),
                RelationShape {
                    field_name: field.name.clone(),
                    field_metadata_id: field.id,
                    relation_type,
                    target_object_metadata_id: field.relation_target_object_metadata_id,
                    target_field_metadata_id: field.relation_target_field_metadata_id,
                    join_column_name: join_column_name.clone(),
                },
            );

            if let Some(column_name) = join_column_name {
                push_column(ColumnShape {
                    column_name: column_name.clone(),
                    field_metadata_id: field.id,
                    field_name: field.name.clone(),
                    field_metadata_type: field.field_type,
                    composite_parent_field_name: None,
                });
            }

            continue;
        }

        if is_composite_field_metadata_type(ddl_field_type(field.field_type)) {
            for property in composite_properties(ddl_field_type(field.field_type)) {
                let column_name = compute_composite_column_name(&field.name, property.name);
                let property_type = FieldMetadataType::parse(property.field_type.as_str())
                    .unwrap_or(field.field_type);

                push_column(ColumnShape {
                    column_name: column_name.clone(),
                    field_metadata_id: field.id,
                    field_name: field.name.clone(),
                    field_metadata_type: property_type,
                    composite_parent_field_name: Some(field.name.clone()),
                });
            }

            continue;
        }

        push_column(ColumnShape {
            column_name: field.name.clone(),
            field_metadata_id: field.id,
            field_name: field.name.clone(),
            field_metadata_type: field.field_type,
            composite_parent_field_name: None,
        });
    }

    let has_deleted_at_column = column_shape_by_column_name.contains_key("deletedAt");
    let columns = column_names
        .iter()
        .filter_map(|name| column_shape_by_column_name.get(name).cloned())
        .collect();

    TableShape {
        object_metadata_id: flat_object_metadata.id,
        name_singular: flat_object_metadata.name_singular.clone(),
        schema_name: nudge_core::ids::workspace_schema_name(workspace_id),
        table_name: target_table_name(flat_object_metadata),
        columns,
        column_names,
        column_shape_by_column_name,
        relation_shape_by_field_name,
        has_deleted_at_column,
    }
}

pub fn object_fields<'a>(
    flat_object_metadata: &'a FlatObjectMetadata,
    flat_field_metadata_maps: &'a FlatEntityMaps<FlatFieldMetadata>,
) -> Vec<&'a FlatFieldMetadata> {
    flat_object_metadata
        .field_ids
        .iter()
        .filter_map(|id| flat_field_metadata_maps.find_by_id(id))
        .collect()
}

pub fn object_fields_or_unknown_object<'a>(
    flat_object_metadata: &'a FlatObjectMetadata,
    flat_field_metadata_maps: &'a FlatEntityMaps<FlatFieldMetadata>,
) -> Result<Vec<&'a FlatFieldMetadata>, RepositoryError> {
    let fields = object_fields(flat_object_metadata, flat_field_metadata_maps);

    if fields.len() != flat_object_metadata.field_ids.len() {
        return Err(RepositoryError::unknown_object(
            &flat_object_metadata.name_singular,
        ));
    }

    Ok(fields)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableShapeKey {
    pub workspace_id: Uuid,
    pub metadata_version: i32,
    pub object_metadata_id: Uuid,
}

#[derive(Default)]
struct LruState {
    entries: HashMap<TableShapeKey, Arc<TableShape>>,
    recency: VecDeque<TableShapeKey>,
}

pub struct TableShapeCache {
    capacity: usize,
    state: Mutex<LruState>,
}

impl Default for TableShapeCache {
    fn default() -> Self {
        Self::new(DEFAULT_TABLE_SHAPE_CACHE_CAPACITY)
    }
}

impl TableShapeCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Mutex::new(LruState::default()),
        }
    }

    pub fn get(&self, key: &TableShapeKey) -> Option<Arc<TableShape>> {
        let mut state = self.state.lock().ok()?;
        let shape = state.entries.get(key).cloned()?;
        touch(&mut state, key);
        Some(shape)
    }

    pub fn insert(&self, key: TableShapeKey, shape: Arc<TableShape>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        state.entries.insert(key.clone(), shape);
        touch(&mut state, &key);

        while state.entries.len() > self.capacity {
            match state.recency.pop_front() {
                Some(evicted) => {
                    state.entries.remove(&evicted);
                }
                None => break,
            }
        }
    }

    pub fn get_or_build(
        &self,
        workspace_id: &Uuid,
        metadata_version: i32,
        flat_object_metadata: &FlatObjectMetadata,
        flat_field_metadata_maps: &FlatEntityMaps<FlatFieldMetadata>,
    ) -> Result<Arc<TableShape>, RepositoryError> {
        let key = TableShapeKey {
            workspace_id: *workspace_id,
            metadata_version,
            object_metadata_id: flat_object_metadata.id,
        };

        if let Some(shape) = self.get(&key) {
            return Ok(shape);
        }

        object_fields_or_unknown_object(flat_object_metadata, flat_field_metadata_maps)?;

        let shape = Arc::new(build_workspace_table_shape(
            workspace_id,
            flat_object_metadata,
            flat_field_metadata_maps,
        ));

        self.insert(key, shape.clone());
        Ok(shape)
    }

    pub fn invalidate_workspace(&self, workspace_id: &Uuid) -> usize {
        let Ok(mut state) = self.state.lock() else {
            return 0;
        };

        let doomed: Vec<TableShapeKey> = state
            .entries
            .keys()
            .filter(|key| key.workspace_id == *workspace_id)
            .cloned()
            .collect();

        let removed = doomed.len();
        for key in doomed {
            state.entries.remove(&key);
            if let Some(position) = state.recency.iter().position(|candidate| *candidate == key) {
                state.recency.remove(position);
            }
        }

        removed
    }

    pub fn len(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.entries.len())
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn touch(state: &mut LruState, key: &TableShapeKey) {
    if let Some(position) = state.recency.iter().position(|candidate| candidate == key) {
        state.recency.remove(position);
    }
    state.recency.push_back(key.clone());
}

fn fixture_field(
    sequence: u8,
    field_type: FieldMetadataType,
    name: &str,
    settings: Option<Value>,
    morph_id: Option<Uuid>,
    target_object: Option<Uuid>,
) -> FlatFieldMetadata {
    let id = Uuid::parse_str(&format!("20202020-0000-4000-8000-00000000000{sequence}"))
        .expect("fixture uuid");

    FlatFieldMetadata {
        id,
        universal_identifier: id.to_string(),
        object_metadata_id: Uuid::nil(),
        object_metadata_universal_identifier: "person-object-id".to_string(),
        field_type,
        name: name.to_string(),
        label: name.to_string(),
        settings,
        morph_id,
        relation_target_object_metadata_id: target_object,
        relation_target_field_metadata_id: None,
        ..Default::default()
    }
}

fn person_fixture_maps() -> FlatEntityMaps<FlatFieldMetadata> {
    let mut maps: FlatEntityMaps<FlatFieldMetadata> = FlatEntityMaps::default();
    let many = serde_json::json!({ "relationType": "MANY_TO_ONE" });
    let one = serde_json::json!({ "relationType": "ONE_TO_MANY" });
    let head_id = Uuid::parse_str("20202020-0000-4000-8000-000000000006").expect("fixture uuid");

    let fields = [
        fixture_field(1, FieldMetadataType::Uuid, "id", None, None, None),
        fixture_field(2, FieldMetadataType::FullName, "name", None, None, None),
        fixture_field(
            3,
            FieldMetadataType::DateTime,
            "deletedAt",
            None,
            None,
            None,
        ),
        fixture_field(
            4,
            FieldMetadataType::Relation,
            "company",
            Some(many.clone()),
            None,
            Some(Uuid::max()),
        ),
        fixture_field(
            5,
            FieldMetadataType::Relation,
            "pets",
            Some(one),
            None,
            Some(Uuid::max()),
        ),
        fixture_field(
            6,
            FieldMetadataType::MorphRelation,
            "target",
            Some(many.clone()),
            Some(head_id),
            None,
        ),
        fixture_field(
            7,
            FieldMetadataType::Relation,
            "targetPerson",
            Some(many),
            Some(head_id),
            Some(Uuid::max()),
        ),
    ];

    for field in fields {
        let (id, universal_identifier) = (field.id.to_string(), field.universal_identifier.clone());
        maps.add(id, field, universal_identifier, "application-id");
    }

    maps
}

fn person_fixture_object(application: &str) -> FlatObjectMetadata {
    FlatObjectMetadata {
        id: Uuid::nil(),
        universal_identifier: "person-object-id".to_string(),
        name_singular: "person".to_string(),
        name_plural: "people".to_string(),
        label_singular: "Person".to_string(),
        label_plural: "People".to_string(),
        application_universal_identifier: application.to_string(),
        target_table_name: Some("DEPRECATED".to_string()),
        field_ids: [
            "20202020-0000-4000-8000-000000000001",
            "20202020-0000-4000-8000-000000000002",
            "20202020-0000-4000-8000-000000000003",
            "20202020-0000-4000-8000-000000000004",
            "20202020-0000-4000-8000-000000000005",
            "20202020-0000-4000-8000-000000000006",
            "20202020-0000-4000-8000-000000000007",
        ]
        .iter()
        .map(|id| id.to_string())
        .collect(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard_person_shape() -> TableShape {
        build_workspace_table_shape(
            &Uuid::nil(),
            &person_fixture_object(
                crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER,
            ),
            &person_fixture_maps(),
        )
    }

    #[test]
    fn the_shape_qualifies_the_workspace_schema_and_standard_table() {
        let shape = standard_person_shape();

        assert!(shape.schema_name.starts_with("workspace_"));
        assert_eq!(shape.table_name, "person");
        assert!(shape.has_deleted_at_column);
    }

    #[test]
    fn a_custom_object_table_carries_the_custom_prefix() {
        let shape = build_workspace_table_shape(
            &Uuid::nil(),
            &person_fixture_object("a-custom-application"),
            &person_fixture_maps(),
        );

        assert_eq!(shape.table_name, "_person");
    }

    #[test]
    fn composite_fields_expand_into_one_column_per_property() {
        let shape = standard_person_shape();

        assert!(shape.column_names.contains(&"nameFirstName".to_string()));
        assert!(shape.column_names.contains(&"nameLastName".to_string()));
        assert!(!shape.column_names.contains(&"name".to_string()));

        let first_name = shape.column("nameFirstName").expect("nameFirstName");
        assert_eq!(
            first_name.composite_parent_field_name.as_deref(),
            Some("name")
        );
        assert_eq!(first_name.field_metadata_type, FieldMetadataType::Text);
    }

    #[test]
    fn a_to_one_relation_carries_a_join_column_and_a_to_many_does_not() {
        let shape = standard_person_shape();

        assert!(shape.column_names.contains(&"companyId".to_string()));
        assert!(!shape.column_names.contains(&"petsId".to_string()));

        let company = shape.relation("company").expect("company relation");
        assert_eq!(company.join_column_name.as_deref(), Some("companyId"));
        assert_eq!(company.relation_type, RelationType::ManyToOne);

        let pets = shape.relation("pets").expect("pets relation");
        assert_eq!(pets.join_column_name, None);
        assert_eq!(pets.relation_type, RelationType::OneToMany);
    }

    #[test]
    fn a_morph_head_is_excluded_and_a_morph_child_is_a_relation_with_a_join_column() {
        let shape = standard_person_shape();

        assert!(shape.relation("target").is_none());
        assert!(!shape.column_names.contains(&"targetId".to_string()));

        let child = shape
            .relation("targetPerson")
            .expect("morph child relation");
        assert_eq!(child.join_column_name.as_deref(), Some("targetPersonId"));
        assert!(shape.column_names.contains(&"targetPersonId".to_string()));
    }

    #[test]
    fn a_relation_without_settings_relation_type_is_skipped() {
        let mut maps = person_fixture_maps();
        let mut untyped = maps
            .find_by_id("20202020-0000-4000-8000-000000000004")
            .expect("company field")
            .clone();
        untyped.settings = None;
        maps.add(
            untyped.id.to_string(),
            untyped,
            "20202020-0000-4000-8000-000000000004",
            "application-id",
        );

        let shape = build_workspace_table_shape(
            &Uuid::nil(),
            &person_fixture_object(
                crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER,
            ),
            &maps,
        );

        assert!(shape.relation("company").is_none());
        assert!(!shape.column_names.contains(&"companyId".to_string()));
    }

    #[test]
    fn the_shape_matches_the_upstream_reference_fixture() {
        let shape = standard_person_shape();

        let expected_columns = [
            "id",
            "nameFirstName",
            "nameLastName",
            "deletedAt",
            "companyId",
            "targetPersonId",
        ];
        for column in expected_columns {
            assert!(
                shape.column_names.iter().any(|name| name == column),
                "missing column {column}"
            );
        }
        assert_eq!(shape.column_names.len(), expected_columns.len());
    }

    #[test]
    fn the_cache_returns_the_same_arc_until_the_version_moves() {
        let cache = TableShapeCache::new(8);
        let maps = person_fixture_maps();
        let object = person_fixture_object(
            crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER,
        );

        let first = cache
            .get_or_build(&Uuid::nil(), 1, &object, &maps)
            .expect("build shape");
        let second = cache
            .get_or_build(&Uuid::nil(), 1, &object, &maps)
            .expect("reuse shape");
        assert!(Arc::ptr_eq(&first, &second));

        let moved = cache
            .get_or_build(&Uuid::nil(), 2, &object, &maps)
            .expect("rebuild after version bump");
        assert!(!Arc::ptr_eq(&first, &moved));
        assert_eq!(cache.len(), 2);

        assert_eq!(cache.invalidate_workspace(&Uuid::nil()), 2);
        assert!(cache.is_empty());
    }

    #[test]
    fn the_cache_evicts_the_least_recently_used_entry() {
        let cache = TableShapeCache::new(2);

        let make_key = |byte: u8| TableShapeKey {
            workspace_id: Uuid::from_bytes([byte; 16]),
            metadata_version: 1,
            object_metadata_id: Uuid::from_bytes([byte; 16]),
        };
        let make_shape = |byte: u8| {
            Arc::new(TableShape {
                object_metadata_id: Uuid::from_bytes([byte; 16]),
                name_singular: "x".to_string(),
                schema_name: "ws".to_string(),
                table_name: "x".to_string(),
                columns: Vec::new(),
                column_names: Vec::new(),
                column_shape_by_column_name: HashMap::new(),
                relation_shape_by_field_name: HashMap::new(),
                has_deleted_at_column: false,
            })
        };

        cache.insert(make_key(1), make_shape(1));
        cache.insert(make_key(2), make_shape(2));
        cache.get(&make_key(1));
        cache.insert(make_key(3), make_shape(3));

        assert!(cache.get(&make_key(1)).is_some());
        assert!(cache.get(&make_key(2)).is_none());
        assert!(cache.get(&make_key(3)).is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn an_unknown_object_field_maps_to_unknown_object() {
        let object = FlatObjectMetadata {
            field_ids: vec!["missing-field-id".to_string()],
            ..person_fixture_object(
                crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER,
            )
        };

        let error = object_fields_or_unknown_object(&object, &person_fixture_maps())
            .expect_err("unknown field id must fail");
        assert_eq!(error.code(), "UNKNOWN_OBJECT");
    }
}
