#![allow(dead_code)]

use std::collections::HashMap;
use std::collections::HashSet;

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::OrchestratorActionsReport;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalCreateAction;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::actions::UniversalUpdateAction;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalFlatFieldMetadata;

const SEARCH_VECTOR_FIELD_NAME: &str = "searchVector";

pub fn aggregate_orchestrator_actions_report(
    mut actions_report: OrchestratorActionsReport,
    flat_field_metadata_maps: &UniversalFlatEntityMaps<UniversalFlatFieldMetadata>,
    search_vector_universal_identifiers_to_rebuild: &HashSet<String>,
) -> OrchestratorActionsReport {
    aggregate_non_relation_fields_into_object_actions(&mut actions_report);
    aggregate_relation_field_pairs(&mut actions_report);
    deprioritize_ts_vector_create_field_actions(&mut actions_report);
    deprioritize_search_vector_update_field_actions(
        &mut actions_report,
        flat_field_metadata_maps,
        search_vector_universal_identifiers_to_rebuild,
    );

    actions_report
}

fn create_actions(
    actions_report: &OrchestratorActionsReport,
    metadata_name: MetadataName,
) -> Vec<UniversalCreateAction> {
    actions_report
        .actions_for(metadata_name)
        .create
        .iter()
        .filter_map(|action| action.clone().into_create())
        .collect()
}

fn update_actions(
    actions_report: &OrchestratorActionsReport,
    metadata_name: MetadataName,
) -> Vec<UniversalUpdateAction> {
    actions_report
        .actions_for(metadata_name)
        .update
        .iter()
        .filter_map(|action| action.clone().into_update())
        .collect()
}

fn flat_field_of(action: &UniversalCreateAction) -> Option<&UniversalFlatFieldMetadata> {
    match action.flat_entity.as_ref()? {
        UniversalFlatEntity::FieldMetadata(field) => Some(field),
        _ => None,
    }
}

fn aggregate_non_relation_fields_into_object_actions(
    actions_report: &mut OrchestratorActionsReport,
) {
    let mut create_object_actions = create_actions(actions_report, MetadataName::ObjectMetadata);

    let mut create_field_actions = create_actions(actions_report, MetadataName::FieldMetadata);

    let mut object_order: Vec<String> = Vec::new();
    let mut create_object_by_universal_identifier: HashMap<String, UniversalCreateAction> =
        HashMap::new();

    for create_object_action in create_object_actions.drain(..) {
        if let Some(flat_entity) = &create_object_action.flat_entity {
            let universal_identifier = flat_entity.universal_identifier().to_string();

            object_order.push(universal_identifier.clone());
            create_object_by_universal_identifier
                .insert(universal_identifier, create_object_action);
        }
    }

    let mut remaining_create_field_actions: Vec<UniversalCreateAction> = Vec::new();

    for create_field_action in create_field_actions.drain(..) {
        let Some(flat_field_metadata) = flat_field_of(&create_field_action) else {
            remaining_create_field_actions.push(create_field_action);

            continue;
        };

        if flat_field_metadata.field_type.is_relation_or_morph() {
            remaining_create_field_actions.push(create_field_action);

            continue;
        }

        let field_universal_identifier = flat_field_metadata.universal_identifier.clone();
        let object_universal_id = flat_field_metadata
            .object_metadata_universal_identifier
            .clone();

        let Some(matching_object_action) =
            create_object_by_universal_identifier.get_mut(&object_universal_id)
        else {
            remaining_create_field_actions.push(create_field_action);

            continue;
        };

        matching_object_action
            .universal_flat_field_metadatas
            .push(flat_field_metadata.clone());

        if let Some(field_id) = &create_field_action.id {
            matching_object_action
                .field_id_by_universal_identifier
                .get_or_insert_with(HashMap::new)
                .insert(field_universal_identifier, field_id.clone());
        }
    }

    let object_bucket = actions_report.actions_for_mut(MetadataName::ObjectMetadata);

    object_bucket.create = object_order
        .into_iter()
        .filter_map(|universal_identifier| {
            create_object_by_universal_identifier
                .remove(&universal_identifier)
                .map(UniversalAction::Create)
        })
        .collect();

    let field_bucket = actions_report.actions_for_mut(MetadataName::FieldMetadata);

    field_bucket.create = remaining_create_field_actions
        .into_iter()
        .map(UniversalAction::Create)
        .collect();
}

fn aggregate_relation_field_pairs(actions_report: &mut OrchestratorActionsReport) {
    let create_field_actions = create_actions(actions_report, MetadataName::FieldMetadata);

    let mut action_by_field_universal_identifier: HashMap<String, UniversalCreateAction> =
        HashMap::new();

    for action in &create_field_actions {
        if let Some(flat_entity) = &action.flat_entity {
            action_by_field_universal_identifier.insert(
                flat_entity.universal_identifier().to_string(),
                action.clone(),
            );
        }
    }

    let mut processed_field_universal_identifiers: HashSet<String> = HashSet::new();
    let mut aggregated_create_field_actions: Vec<UniversalCreateAction> = Vec::new();

    for action in &create_field_actions {
        let Some(universal_identifier) = action.flat_entity.as_ref().map(|entity| entity.universal_identifier().to_string()) else {
            aggregated_create_field_actions.push(action.clone());

            continue;
        };

        if processed_field_universal_identifiers.contains(&universal_identifier) {
            continue;
        }

        processed_field_universal_identifiers.insert(universal_identifier);

        let target_universal_identifier = flat_field_of(action).and_then(|field| {
            field
                .relation_target_field_metadata_universal_identifier
                .clone()
        });

        let Some(target_universal_identifier) = target_universal_identifier else {
            aggregated_create_field_actions.push(action.clone());

            continue;
        };

        let target_action = action_by_field_universal_identifier.get(&target_universal_identifier);

        let Some(target_action) = target_action else {
            aggregated_create_field_actions.push(action.clone());

            continue;
        };

        if processed_field_universal_identifiers.contains(&target_universal_identifier) {
            aggregated_create_field_actions.push(action.clone());

            continue;
        }

        processed_field_universal_identifiers.insert(target_universal_identifier);

        let mut merged = action.clone();

        merged.related_universal_flat_field_metadata = flat_field_of(target_action).cloned();
        merged.related_field_id = target_action.id.clone();

        aggregated_create_field_actions.push(merged);
    }

    let field_bucket = actions_report.actions_for_mut(MetadataName::FieldMetadata);

    field_bucket.create = aggregated_create_field_actions
        .into_iter()
        .map(UniversalAction::Create)
        .collect();
}

fn deprioritize_ts_vector_create_field_actions(actions_report: &mut OrchestratorActionsReport) {
    let create_field_actions = create_actions(actions_report, MetadataName::FieldMetadata);

    let is_ts_vector = |action: &UniversalCreateAction| -> bool {
        flat_field_of(action)
            .map(|field| field.field_type == crate::metadata::flat::FieldMetadataType::TsVector)
            .unwrap_or(false)
    };

    let ts_vector_create_field_actions: Vec<UniversalCreateAction> = create_field_actions
        .iter()
        .filter(|action| is_ts_vector(action))
        .cloned()
        .collect();

    if ts_vector_create_field_actions.is_empty() {
        return;
    }

    let other_create_field_actions: Vec<UniversalCreateAction> = create_field_actions
        .iter()
        .filter(|action| !is_ts_vector(action))
        .cloned()
        .collect();

    let mut reordered = other_create_field_actions;
    reordered.extend(ts_vector_create_field_actions);

    let field_bucket = actions_report.actions_for_mut(MetadataName::FieldMetadata);

    field_bucket.create = reordered.into_iter().map(UniversalAction::Create).collect();
}

fn deprioritize_search_vector_update_field_actions(
    actions_report: &mut OrchestratorActionsReport,
    flat_field_metadata_maps: &UniversalFlatEntityMaps<UniversalFlatFieldMetadata>,
    rebuild_target_universal_identifiers: &HashSet<String>,
) {
    let update_field_actions = update_actions(actions_report, MetadataName::FieldMetadata);

    let mut search_vector_update_field_actions: Vec<UniversalUpdateAction> = Vec::new();
    let mut other_update_field_actions: Vec<UniversalUpdateAction> = Vec::new();
    let mut covered_vector_universal_identifiers: HashSet<String> = HashSet::new();

    for update_field_action in update_field_actions {
        let flat_field_metadata = flat_field_metadata_maps
            .find_by_universal_identifier(&update_field_action.universal_identifier);

        let is_search_vector_update_field_action = flat_field_metadata
            .map(|field| field.name == SEARCH_VECTOR_FIELD_NAME)
            .unwrap_or(false);

        if is_search_vector_update_field_action {
            covered_vector_universal_identifiers
                .insert(update_field_action.universal_identifier.clone());

            let mut action = update_field_action;

            if rebuild_target_universal_identifiers.contains(&action.universal_identifier) {
                action.rebuild_search_vector = Some(true);
            }

            search_vector_update_field_actions.push(action);
        } else {
            other_update_field_actions.push(update_field_action);
        }
    }

    let synthesized_search_vector_update_field_actions: Vec<UniversalUpdateAction> =
        rebuild_target_universal_identifiers
            .iter()
            .filter(|universal_identifier| {
                !covered_vector_universal_identifiers.contains(*universal_identifier)
            })
            .map(|universal_identifier| UniversalUpdateAction {
                metadata_name: MetadataName::FieldMetadata,
                universal_identifier: universal_identifier.clone(),
                update: serde_json::Value::Object(serde_json::Map::new()),
                rebuild_search_vector: Some(true),
                ..UniversalUpdateAction::default()
            })
            .collect();

    let mut reordered = other_update_field_actions;
    reordered.extend(search_vector_update_field_actions);
    reordered.extend(synthesized_search_vector_update_field_actions);

    let field_bucket = actions_report.actions_for_mut(MetadataName::FieldMetadata);

    field_bucket.update = reordered.into_iter().map(UniversalAction::Update).collect();
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::aggregate_orchestrator_actions_report;
    use crate::metadata::actions::MetadataName;
    use crate::metadata::actions::OrchestratorActionsReport;
    use crate::metadata::actions::UniversalAction;
    use crate::metadata::actions::UniversalCreateAction;
    use crate::metadata::actions::UniversalFlatEntity;
    use crate::metadata::actions::UniversalUpdateAction;
    use crate::metadata::flat::FieldMetadataType;
    use crate::metadata::flat::UniversalFlatFieldMetadata;
    use crate::metadata::flat::UniversalFlatObjectMetadata;

    fn field_entity(
        universal_identifier: &str,
        object_universal_identifier: &str,
        field_type: FieldMetadataType,
        name: &str,
    ) -> UniversalFlatFieldMetadata {
        UniversalFlatFieldMetadata {
            universal_identifier: universal_identifier.to_string(),
            object_metadata_universal_identifier: object_universal_identifier.to_string(),
            field_type,
            name: name.to_string(),
            label: name.to_string(),
            relation_target_field_metadata_universal_identifier: None,
            ..UniversalFlatFieldMetadata::default()
        }
    }

    fn relation_field_entity(
        universal_identifier: &str,
        object_universal_identifier: &str,
        target_field_universal_identifier: &str,
        name: &str,
    ) -> UniversalFlatFieldMetadata {
        UniversalFlatFieldMetadata {
            universal_identifier: universal_identifier.to_string(),
            object_metadata_universal_identifier: object_universal_identifier.to_string(),
            field_type: FieldMetadataType::Relation,
            name: name.to_string(),
            label: name.to_string(),
            relation_target_field_metadata_universal_identifier: Some(
                target_field_universal_identifier.to_string(),
            ),
            ..UniversalFlatFieldMetadata::default()
        }
    }

    fn object_create_action(universal_identifier: &str) -> UniversalAction {
        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::ObjectMetadata,
            flat_entity: Some(UniversalFlatEntity::ObjectMetadata(Box::new(
                UniversalFlatObjectMetadata {
                    universal_identifier: universal_identifier.to_string(),
                    name_singular: format!("object_{universal_identifier}"),
                    ..UniversalFlatObjectMetadata::default()
                },
            ))),
            universal_flat_field_metadatas: Vec::new(),
            ..UniversalCreateAction::default()
        })
    }

    fn field_create_action(field: UniversalFlatFieldMetadata, id: Option<&str>) -> UniversalAction {
        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::FieldMetadata,
            flat_entity: Some(UniversalFlatEntity::FieldMetadata(Box::new(field))),
            id: id.map(str::to_string),
            ..UniversalCreateAction::default()
        })
    }

    fn report_with(
        object_creates: Vec<UniversalAction>,
        field_creates: Vec<UniversalAction>,
    ) -> OrchestratorActionsReport {
        let mut report = OrchestratorActionsReport::default();

        report
            .actions_for_mut(MetadataName::ObjectMetadata)
            .create
            .extend(object_creates);
        report
            .actions_for_mut(MetadataName::FieldMetadata)
            .create
            .extend(field_creates);

        report
    }

    #[test]
    fn non_relation_fields_fold_into_matching_object_creates() {
        let report = report_with(
            vec![object_create_action("object-1")],
            vec![field_create_action(
                field_entity("field-1", "object-1", FieldMetadataType::Text, "textField"),
                Some("generated-field-id"),
            )],
        );

        let field_maps =
            crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default();
        let aggregated =
            aggregate_orchestrator_actions_report(report, &field_maps, &HashSet::new());

        let object_creates = aggregated
            .actions_for(MetadataName::ObjectMetadata)
            .create
            .clone();
        let field_creates = aggregated
            .actions_for(MetadataName::FieldMetadata)
            .create
            .clone();

        assert_eq!(field_creates.len(), 0);

        let folded = object_creates[0].clone().into_create().unwrap();

        assert_eq!(folded.universal_flat_field_metadatas.len(), 1);
        assert_eq!(
            folded.universal_flat_field_metadatas[0].universal_identifier,
            "field-1"
        );
        assert_eq!(
            folded
                .field_id_by_universal_identifier
                .as_ref()
                .and_then(|map| map.get("field-1"))
                .map(String::as_str),
            Some("generated-field-id")
        );
    }

    #[test]
    fn relation_fields_stay_in_separate_create_actions() {
        let report = report_with(
            vec![object_create_action("object-1")],
            vec![field_create_action(
                relation_field_entity("relation-1", "object-1", "target-1", "relationField"),
                Some("relation-field-id"),
            )],
        );

        let field_maps =
            crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default();
        let aggregated =
            aggregate_orchestrator_actions_report(report, &field_maps, &HashSet::new());

        assert_eq!(
            aggregated
                .actions_for(MetadataName::ObjectMetadata)
                .create
                .iter()
                .filter_map(|action| action.clone().into_create())
                .map(|create| create.universal_flat_field_metadatas.len())
                .sum::<usize>(),
            0
        );
        assert_eq!(
            aggregated
                .actions_for(MetadataName::FieldMetadata)
                .create
                .len(),
            1
        );
    }

    #[test]
    fn fields_of_existing_objects_stay_separate() {
        let report = report_with(
            Vec::new(),
            vec![field_create_action(
                field_entity(
                    "field-1",
                    "existing-object",
                    FieldMetadataType::Text,
                    "textField",
                ),
                Some("generated-field-id"),
            )],
        );

        let field_maps =
            crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default();
        let aggregated =
            aggregate_orchestrator_actions_report(report, &field_maps, &HashSet::new());

        assert_eq!(
            aggregated
                .actions_for(MetadataName::ObjectMetadata)
                .create
                .len(),
            0
        );
        assert_eq!(
            aggregated
                .actions_for(MetadataName::FieldMetadata)
                .create
                .len(),
            1
        );
    }

    #[test]
    fn relation_field_pairs_merge_into_one_action() {
        let report = report_with(
            Vec::new(),
            vec![
                field_create_action(
                    relation_field_entity("owner-1", "object-1", "target-1", "relationField"),
                    Some("owner-id"),
                ),
                field_create_action(
                    relation_field_entity("target-1", "object-2", "owner-1", "relationTarget"),
                    Some("target-id"),
                ),
            ],
        );

        let field_maps =
            crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default();
        let aggregated =
            aggregate_orchestrator_actions_report(report, &field_maps, &HashSet::new());

        let field_creates = aggregated
            .actions_for(MetadataName::FieldMetadata)
            .create
            .clone();

        assert_eq!(field_creates.len(), 1);

        let paired = field_creates[0].clone().into_create().unwrap();

        assert_eq!(
            paired
                .flat_entity
                .as_ref()
                .map(|entity| entity.universal_identifier()),
            Some("owner-1")
        );
        assert_eq!(
            paired
                .related_universal_flat_field_metadata
                .as_ref()
                .map(|field| field.universal_identifier.as_str()),
            Some("target-1")
        );
        assert_eq!(paired.related_field_id.as_deref(), Some("target-id"));
    }

    #[test]
    fn unpaired_relation_fields_are_kept_as_is() {
        let report = report_with(
            Vec::new(),
            vec![field_create_action(
                relation_field_entity("owner-1", "object-1", "missing-target", "relationField"),
                None,
            )],
        );

        let field_maps =
            crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default();
        let aggregated =
            aggregate_orchestrator_actions_report(report, &field_maps, &HashSet::new());

        assert_eq!(
            aggregated
                .actions_for(MetadataName::FieldMetadata)
                .create
                .len(),
            1
        );
    }

    #[test]
    fn ts_vector_creates_run_after_other_field_creates() {
        let report = report_with(
            Vec::new(),
            vec![
                field_create_action(
                    field_entity(
                        "ts-vector-1",
                        "object-1",
                        FieldMetadataType::TsVector,
                        "searchVector",
                    ),
                    None,
                ),
                field_create_action(
                    field_entity("text-1", "object-1", FieldMetadataType::Text, "textField"),
                    None,
                ),
            ],
        );

        let field_maps =
            crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default();
        let aggregated =
            aggregate_orchestrator_actions_report(report, &field_maps, &HashSet::new());

        let field_creates = aggregated
            .actions_for(MetadataName::FieldMetadata)
            .create
            .clone();

        let ordered_universal_identifiers: Vec<String> = field_creates
            .iter()
            .filter_map(|action| {
                action
                    .clone()
                    .into_create()
                    .and_then(|create| create.flat_entity)
                    .map(|entity| entity.universal_identifier().to_string())
            })
            .collect();

        assert_eq!(ordered_universal_identifiers, vec!["text-1", "ts-vector-1"]);
    }

    #[test]
    fn search_vector_updates_are_deprioritized_and_rebuild_flags_are_injected() {
        let mut report = OrchestratorActionsReport::default();

        report
            .actions_for_mut(MetadataName::FieldMetadata)
            .update
            .push(UniversalAction::Update(UniversalUpdateAction {
                metadata_name: MetadataName::FieldMetadata,
                universal_identifier: "plain-field".to_string(),
                update: serde_json::json!({ "label": "New" }),
                ..UniversalUpdateAction::default()
            }));

        report
            .actions_for_mut(MetadataName::FieldMetadata)
            .update
            .push(UniversalAction::Update(UniversalUpdateAction {
                metadata_name: MetadataName::FieldMetadata,
                universal_identifier: "search-vector".to_string(),
                update: serde_json::json!({}),
                ..UniversalUpdateAction::default()
            }));

        let mut field_maps =
            crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default();

        field_maps.add(
            "search-vector".to_string(),
            UniversalFlatFieldMetadata {
                universal_identifier: "search-vector".to_string(),
                name: "searchVector".to_string(),
                field_type: FieldMetadataType::TsVector,
                ..UniversalFlatFieldMetadata::default()
            },
        );

        let mut rebuild_targets = HashSet::new();
        rebuild_targets.insert("search-vector".to_string());
        rebuild_targets.insert("missing-vector".to_string());

        let aggregated =
            aggregate_orchestrator_actions_report(report, &field_maps, &rebuild_targets);

        let update_universal_identifiers: Vec<String> = aggregated
            .actions_for(MetadataName::FieldMetadata)
            .update
            .iter()
            .filter_map(|action| {
                action
                    .clone()
                    .into_update()
                    .map(|update| update.universal_identifier)
            })
            .collect();

        assert_eq!(
            update_universal_identifiers,
            vec!["plain-field", "search-vector", "missing-vector"]
        );

        let rebuild_update = aggregated
            .actions_for(MetadataName::FieldMetadata)
            .update
            .iter()
            .filter_map(|action| action.clone().into_update())
            .find(|update| update.universal_identifier == "search-vector")
            .unwrap();

        assert_eq!(rebuild_update.rebuild_search_vector, Some(true));

        let synthesized = aggregated
            .actions_for(MetadataName::FieldMetadata)
            .update
            .iter()
            .filter_map(|action| action.clone().into_update())
            .find(|update| update.universal_identifier == "missing-vector")
            .unwrap();

        assert_eq!(synthesized.rebuild_search_vector, Some(true));
        assert_eq!(synthesized.update, serde_json::json!({}));
    }

    #[test]
    fn rebuild_targets_skip_fields_being_created_or_deleted() {
        use crate::metadata::actions::UniversalDeleteAction;
        use crate::metadata::flat::UniversalFlatSearchFieldMetadata;
        use crate::metadata::validate::compute_search_vector_rebuild_target_universal_identifiers;

        let mut report = OrchestratorActionsReport::default();

        report
            .actions_for_mut(MetadataName::FieldMetadata)
            .create
            .push(field_create_action(
                field_entity(
                    "created-vector",
                    "object-1",
                    FieldMetadataType::TsVector,
                    "searchVector",
                ),
                None,
            ));

        let mut to_search_field_maps = crate::metadata::flat::UniversalFlatEntityMaps::<
            UniversalFlatSearchFieldMetadata,
        >::default();

        to_search_field_maps.add(
            "search-field-1".to_string(),
            UniversalFlatSearchFieldMetadata {
                universal_identifier: "search-field-1".to_string(),
                ts_vector_field_metadata_universal_identifier: "created-vector".to_string(),
                ..UniversalFlatSearchFieldMetadata::default()
            },
        );

        let mut report_with_delete = OrchestratorActionsReport::default();

        report_with_delete
            .actions_for_mut(MetadataName::SearchFieldMetadata)
            .delete
            .push(UniversalAction::Delete(UniversalDeleteAction {
                metadata_name: MetadataName::SearchFieldMetadata,
                universal_identifier: "search-field-1".to_string(),
                ..UniversalDeleteAction::default()
            }));

        let targets = compute_search_vector_rebuild_target_universal_identifiers(
            &report_with_delete,
            &to_search_field_maps,
            &to_search_field_maps,
            &crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default(
            ),
        );

        assert_eq!(targets.len(), 1);
        assert!(targets.contains("created-vector"));

        let with_create = compute_search_vector_rebuild_target_universal_identifiers(
            &report,
            &to_search_field_maps,
            &to_search_field_maps,
            &crate::metadata::flat::UniversalFlatEntityMaps::<UniversalFlatFieldMetadata>::default(
            ),
        );

        assert!(with_create.is_empty());
    }
}
