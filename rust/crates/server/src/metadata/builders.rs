#![allow(dead_code)]

pub mod field;
pub mod field_option;
pub mod index;
pub mod matrix;
pub mod object;
pub mod report;
pub mod search_field;

use std::collections::HashMap;
use std::collections::HashSet;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::metadata::actions::comparable_properties;
use crate::metadata::actions::diff_universal_flat_entity_properties;
use crate::metadata::actions::MetadataName;
use crate::metadata::actions::OrchestratorActionsReport;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::actions::WorkspaceMigration;
use crate::metadata::aggregate::aggregate_orchestrator_actions_report;
use crate::metadata::flat::FromToAllMaps;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalWorkspaceMaps;
use crate::metadata::order::compute_ordered_migration_actions;
use crate::metadata::validate::compute_search_vector_rebuild_target_universal_identifiers;
use crate::metadata::validate::cross_entity_transversal_validation;

#[allow(unused_imports)]
pub use matrix::expand_matrix_with_side_effects;
#[allow(unused_imports)]
pub use matrix::FlatEntityOperationsMatrix;
#[allow(unused_imports)]
pub use matrix::FlatEntityToCreateDeleteUpdate;
#[allow(unused_imports)]
pub use report::FailedFlatEntityValidation;
#[allow(unused_imports)]
pub use report::FlatEntityValidationError;
#[allow(unused_imports)]
pub use report::OrchestratorFailureReport;

pub const IDENTIFIER_MIN_CHAR_LENGTH: usize = 1;
pub const IDENTIFIER_MAX_CHAR_LENGTH: usize = 63;
pub const TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER: &str =
    "6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264";

pub const RESERVED_METADATA_NAME_KEYWORDS: &[&str] = &[
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
    "securityToken",
    "securityTokens",
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

pub const TAG_COLORS: &[&str] = &[
    "red",
    "ruby",
    "crimson",
    "tomato",
    "orange",
    "amber",
    "yellow",
    "lime",
    "grass",
    "green",
    "jade",
    "mint",
    "turquoise",
    "cyan",
    "sky",
    "blue",
    "iris",
    "violet",
    "purple",
    "plum",
    "pink",
    "bronze",
    "gold",
    "brown",
    "gray",
];

pub fn universal_foreign_key_aggregator_properties(
    metadata_name: MetadataName,
) -> &'static [&'static str] {
    match metadata_name {
        MetadataName::ObjectMetadata => &[
            "fieldUniversalIdentifiers",
            "indexMetadataUniversalIdentifiers",
            "objectPermissionUniversalIdentifiers",
            "fieldPermissionUniversalIdentifiers",
            "rowLevelPermissionRuleUniversalIdentifiers",
            "viewUniversalIdentifiers",
            "searchFieldMetadataUniversalIdentifiers",
            "pageLayoutUniversalIdentifiers",
            "navigationMenuItemUniversalIdentifiers",
            "commandMenuItemUniversalIdentifiers",
        ],
        MetadataName::FieldMetadata => &[
            "viewSortUniversalIdentifiers",
            "fieldPermissionUniversalIdentifiers",
            "viewFieldUniversalIdentifiers",
            "viewFilterUniversalIdentifiers",
            "kanbanAggregateOperationViewUniversalIdentifiers",
            "calendarViewUniversalIdentifiers",
            "calendarEndViewUniversalIdentifiers",
            "mainGroupByFieldMetadataViewUniversalIdentifiers",
            "searchFieldMetadataUniversalIdentifiers",
            "fieldMetadataOptionUniversalIdentifiers",
            "rowLevelPermissionPredicateUniversalIdentifiers",
            "relationRowLevelPermissionPredicateUniversalIdentifiers",
            "workspaceMemberRowLevelPermissionPredicateUniversalIdentifiers",
        ],
        _ => &[],
    }
}

pub fn flat_foreign_key_drop_properties(metadata_name: MetadataName) -> &'static [&'static str] {
    match metadata_name {
        MetadataName::ObjectMetadata => &[
            "labelIdentifierFieldMetadataId",
            "imageIdentifierFieldMetadataId",
        ],
        MetadataName::FieldMetadata => &[
            "objectMetadataId",
            "relationTargetFieldMetadataId",
            "relationTargetObjectMetadataId",
            "settings",
        ],
        MetadataName::Index => &["objectMetadataId"],
        MetadataName::SearchFieldMetadata => &[
            "objectMetadataId",
            "fieldMetadataId",
            "tsVectorFieldMetadataId",
        ],
        MetadataName::FieldMetadataOption => &["fieldMetadataId"],
        _ => &[],
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferDeletionFromMissingEntities {
    All(bool),
    PerName(HashMap<MetadataName, bool>),
}

impl Default for InferDeletionFromMissingEntities {
    fn default() -> Self {
        InferDeletionFromMissingEntities::All(false)
    }
}

impl InferDeletionFromMissingEntities {
    pub fn for_name(&self, metadata_name: MetadataName) -> bool {
        match self {
            InferDeletionFromMissingEntities::All(value) => *value,
            InferDeletionFromMissingEntities::PerName(map) => {
                map.get(&metadata_name).copied().unwrap_or(false)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildOptions {
    pub is_system_build: bool,
    pub application_universal_identifier: String,
    pub workspace_custom_application_universal_identifier: String,
    pub application_default_role_universal_identifier: Option<String>,
    pub infer_deletion_from_missing_entities: InferDeletionFromMissingEntities,
}

impl BuildOptions {
    pub fn is_caller_twenty_standard_app(&self) -> bool {
        self.application_universal_identifier == TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER
    }
}

pub trait UniversalEntityLike:
    Clone + PartialEq + Serialize + DeserializeOwned + std::fmt::Debug
{
    fn universal_identifier(&self) -> &str;
    fn merge_update(&self, update: &serde_json::Value) -> Result<Self, serde_json::Error>;
    fn to_diff_value(&self) -> serde_json::Value;
}

#[expect(
    clippy::result_large_err,
    reason = "the error mirrors the upstream validation report shape"
)]
pub trait EntityActionsBuilder {
    type Entity: UniversalEntityLike;

    fn metadata_name(&self) -> MetadataName;

    fn maps_of<'m>(
        &self,
        maps: &'m UniversalWorkspaceMaps,
    ) -> &'m UniversalFlatEntityMaps<Self::Entity>;

    fn maps_of_mut<'m>(
        &self,
        maps: &'m mut UniversalWorkspaceMaps,
    ) -> &'m mut UniversalFlatEntityMaps<Self::Entity>;

    fn validate_creation(
        &self,
        context: &CreationValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation>;

    fn validate_deletion(
        &self,
        context: &DeletionValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation>;

    fn validate_update(
        &self,
        context: &UpdateValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation>;
}

pub struct CreationValidationContext<'a, B: EntityActionsBuilder + ?Sized> {
    pub flat_entity_to_validate: B::Entity,
    pub optimistic_maps: &'a UniversalWorkspaceMaps,
    pub remaining_maps: &'a UniversalFlatEntityMaps<B::Entity>,
    pub final_maps: &'a UniversalFlatEntityMaps<B::Entity>,
    pub build_options: &'a BuildOptions,
    pub builder: &'a B,
}

pub struct DeletionValidationContext<'a, B: EntityActionsBuilder + ?Sized> {
    pub flat_entity_to_validate: B::Entity,
    pub optimistic_maps: &'a UniversalWorkspaceMaps,
    pub remaining_maps: &'a UniversalFlatEntityMaps<B::Entity>,
    pub build_options: &'a BuildOptions,
    pub builder: &'a B,
}

pub struct UpdateValidationContext<'a, B: EntityActionsBuilder + ?Sized> {
    pub universal_identifier: &'a str,
    pub flat_entity_update: &'a serde_json::Value,
    pub updated_flat_entity: B::Entity,
    pub optimistic_maps: &'a UniversalWorkspaceMaps,
    pub build_options: &'a BuildOptions,
    pub builder: &'a B,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidateAndBuildOutcome {
    Success {
        create: Vec<UniversalAction>,
        update: Vec<UniversalAction>,
        delete: Vec<UniversalAction>,
    },
    Fail(Vec<FailedFlatEntityValidation>),
}

const SERVER_MANAGED_PROPERTIES: &[&str] = &[
    "createdAt",
    "updatedAt",
    "isSystemSideEffect",
    "createdByUserWorkspaceId",
];

const BASE_WORKSPACE_SCOPED_PROPERTIES: &[&str] = &["id", "workspaceId", "applicationId"];

pub fn validate_and_build<B: EntityActionsBuilder>(
    builder: &B,
    build_options: &BuildOptions,
    optimistic_maps: &mut UniversalWorkspaceMaps,
    from_maps: &UniversalFlatEntityMaps<B::Entity>,
    to_maps: &UniversalFlatEntityMaps<B::Entity>,
) -> ValidateAndBuildOutcome {
    let metadata_name = builder.metadata_name();
    let mut failures: Vec<FailedFlatEntityValidation> = Vec::new();
    let mut delete_actions: Vec<UniversalAction> = Vec::new();
    let mut create_actions: Vec<UniversalAction> = Vec::new();
    let mut update_actions: Vec<UniversalAction> = Vec::new();

    let infer_deletion = build_options
        .infer_deletion_from_missing_entities
        .for_name(metadata_name);

    let mut remaining_deletions: UniversalFlatEntityMaps<B::Entity> =
        UniversalFlatEntityMaps::default();

    if infer_deletion {
        for (universal_identifier, entity) in &from_maps.by_universal_identifier {
            if !to_maps
                .by_universal_identifier
                .contains_key(universal_identifier)
            {
                remaining_deletions.add(universal_identifier.clone(), entity.clone());
            }
        }
    }

    let deletion_universal_identifiers: Vec<String> = remaining_deletions
        .by_universal_identifier
        .keys()
        .cloned()
        .collect();

    for universal_identifier in deletion_universal_identifiers {
        let flat_entity = match from_maps.by_universal_identifier.get(&universal_identifier) {
            Some(entity) => entity.clone(),
            None => continue,
        };

        remaining_deletions.delete_by_universal_identifier(&universal_identifier);

        let context = DeletionValidationContext {
            flat_entity_to_validate: flat_entity.clone(),
            optimistic_maps,
            remaining_maps: &remaining_deletions,
            build_options,
            builder,
        };

        match builder.validate_deletion(&context) {
            Ok(mut action) => {
                attach_delete_entity(metadata_name, &mut action, &flat_entity);
                delete_actions.push(action);
                builder
                    .maps_of_mut(optimistic_maps)
                    .delete_by_universal_identifier(&universal_identifier);
            }
            Err(failure) => failures.push(failure),
        }
    }

    let mut final_maps: UniversalFlatEntityMaps<B::Entity> = UniversalFlatEntityMaps::default();

    for (universal_identifier, entity) in builder
        .maps_of(optimistic_maps)
        .by_universal_identifier
        .iter()
    {
        final_maps.add(universal_identifier.clone(), entity.clone());
    }

    for (universal_identifier, entity) in &to_maps.by_universal_identifier {
        final_maps.add(universal_identifier.clone(), entity.clone());
    }

    let mut remaining_creations: UniversalFlatEntityMaps<B::Entity> =
        UniversalFlatEntityMaps::default();

    for (universal_identifier, entity) in &to_maps.by_universal_identifier {
        if !from_maps
            .by_universal_identifier
            .contains_key(universal_identifier)
        {
            remaining_creations.add(universal_identifier.clone(), entity.clone());
        }
    }

    let creation_order = topologically_sort_creations(metadata_name, &remaining_creations);

    for universal_identifier in creation_order {
        let flat_entity = match remaining_creations
            .by_universal_identifier
            .get(&universal_identifier)
        {
            Some(entity) => entity.clone(),
            None => continue,
        };

        remaining_creations.delete_by_universal_identifier(&universal_identifier);

        let context = CreationValidationContext {
            flat_entity_to_validate: flat_entity.clone(),
            optimistic_maps,
            remaining_maps: &remaining_creations,
            final_maps: &final_maps,
            build_options,
            builder,
        };

        match builder.validate_creation(&context) {
            Ok(mut action) => {
                strip_create_foreign_key_aggregators(metadata_name, &mut action);
                create_actions.push(action);
                builder
                    .maps_of_mut(optimistic_maps)
                    .add(universal_identifier.clone(), flat_entity);
            }
            Err(failure) => failures.push(failure),
        }
    }

    let update_universal_identifiers: Vec<String> = from_maps
        .by_universal_identifier
        .keys()
        .filter(|universal_identifier| {
            to_maps
                .by_universal_identifier
                .contains_key(*universal_identifier)
        })
        .cloned()
        .collect();

    for universal_identifier in update_universal_identifiers {
        let from_entity = &from_maps.by_universal_identifier[&universal_identifier];
        let to_entity = &to_maps.by_universal_identifier[&universal_identifier];

        let comparison = compare_entities(metadata_name, from_entity, to_entity);

        let (update_payload, non_updatable_divergences) = match comparison {
            Some(comparison) => comparison,
            None => continue,
        };

        let updated_entity = match from_entity.merge_update(&update_payload) {
            Ok(updated) => updated,
            Err(error) => {
                failures.push(FailedFlatEntityValidation {
                    validation_type: "update".to_string(),
                    metadata_name: Some(metadata_name),
                    errors: vec![FlatEntityValidationError {
                        code: "BUILDER_INTERNAL_SERVER_ERROR".to_string(),
                        message: format!(
                            "could not merge update for {metadata_name} {universal_identifier}: {error}"
                        ),
                        user_friendly_message: None,
                        value: None,
                    }],
                    flat_entity_minimal_information: serde_json::json!({
                        "universalIdentifier": universal_identifier
                    }),
                });

                continue;
            }
        };

        let context = UpdateValidationContext {
            universal_identifier: &universal_identifier,
            flat_entity_update: &update_payload,
            updated_flat_entity: updated_entity.clone(),
            optimistic_maps,
            build_options,
            builder,
        };

        let centralized_errors = non_updatable_divergences
            .iter()
            .map(|property| FlatEntityValidationError {
                code: "NON_UPDATABLE_PROPERTY_DIVERGENCE".to_string(),
                message: format!("{metadata_name} property \"{property}\" cannot be updated"),
                user_friendly_message: None,
                value: None,
            })
            .collect::<Vec<FlatEntityValidationError>>();

        match builder.validate_update(&context) {
            Ok(mut action) => {
                let diff = diff_universal_flat_entity_properties(
                    metadata_name,
                    &from_entity.to_diff_value(),
                    &to_entity.to_diff_value(),
                );

                if !centralized_errors.is_empty() {
                    failures.push(FailedFlatEntityValidation {
                        validation_type: "update".to_string(),
                        metadata_name: Some(metadata_name),
                        errors: centralized_errors,
                        flat_entity_minimal_information: serde_json::json!({
                            "universalIdentifier": universal_identifier
                        }),
                    });

                    continue;
                }

                attach_update_entity(&mut action, &updated_entity, &diff);
                update_actions.push(action);
                builder
                    .maps_of_mut(optimistic_maps)
                    .add(universal_identifier.clone(), updated_entity);
            }
            Err(mut failure) => {
                failure.errors.extend(centralized_errors);
                failures.push(failure);
            }
        }
    }

    if !failures.is_empty() {
        return ValidateAndBuildOutcome::Fail(failures);
    }

    ValidateAndBuildOutcome::Success {
        create: create_actions,
        update: update_actions,
        delete: delete_actions,
    }
}

fn attach_delete_entity<T: UniversalEntityLike>(
    metadata_name: MetadataName,
    action: &mut UniversalAction,
    entity: &T,
) {
    let mut payload = entity.to_diff_value();

    if let Some(map) = payload.as_object_mut() {
        for base_property in BASE_WORKSPACE_SCOPED_PROPERTIES {
            map.remove(*base_property);
        }

        for property in flat_foreign_key_drop_properties(metadata_name) {
            map.remove(*property);
        }

        let child_id_keys: Vec<String> = map
            .keys()
            .filter(|key| key.ends_with("Ids"))
            .cloned()
            .collect();

        for key in child_id_keys {
            map.remove(&key);
        }
    }

    if let UniversalAction::Delete(delete_action) = action {
        delete_action.flat_entity = Some(UniversalFlatEntity::Other {
            metadata_name,
            entity: payload,
        });
    }
}

fn attach_update_entity<T: UniversalEntityLike>(
    action: &mut UniversalAction,
    entity: &T,
    diff: &serde_json::Value,
) {
    let payload = entity.to_diff_value();

    if let UniversalAction::Update(update_action) = action {
        update_action.flat_entity = Some(UniversalFlatEntity::Other {
            metadata_name: update_action.metadata_name,
            entity: payload,
        });
        update_action.diff = Some(diff.clone());
    }
}

fn strip_create_foreign_key_aggregators(metadata_name: MetadataName, action: &mut UniversalAction) {
    let UniversalAction::Create(create_action) = action else {
        return;
    };

    let Some(flat_entity) = &create_action.flat_entity else {
        return;
    };

    let Ok(mut payload) = flat_entity.to_value() else {
        return;
    };

    if let Some(map) = payload.as_object_mut() {
        for aggregator in universal_foreign_key_aggregator_properties(metadata_name) {
            map.remove(*aggregator);
        }
    }

    create_action.flat_entity = Some(UniversalFlatEntity::Other {
        metadata_name,
        entity: payload,
    });
}

type EntityComparison = (serde_json::Value, Vec<String>);

fn compare_entities<T: UniversalEntityLike>(
    metadata_name: MetadataName,
    from_entity: &T,
    to_entity: &T,
) -> Option<EntityComparison> {
    let from_value = from_entity.to_diff_value();
    let to_value = to_entity.to_diff_value();

    let update = diff_universal_flat_entity_properties(metadata_name, &from_value, &to_value);

    let non_updatable_divergences =
        non_comparable_property_divergences(metadata_name, &from_value, &to_value);

    let update_object = update.as_object()?;

    if update_object.is_empty() && non_updatable_divergences.is_empty() {
        return None;
    }

    Some((update, non_updatable_divergences))
}

fn non_comparable_property_divergences(
    metadata_name: MetadataName,
    from_value: &serde_json::Value,
    to_value: &serde_json::Value,
) -> Vec<String> {
    let comparable = comparable_properties(metadata_name);
    let mut divergences = Vec::new();

    if let (Some(from_object), Some(to_object)) = (from_value.as_object(), to_value.as_object()) {
        for (property, to_property_value) in to_object {
            if property == "universalIdentifier" || property == "applicationUniversalIdentifier" {
                continue;
            }

            if SERVER_MANAGED_PROPERTIES.contains(&property.as_str()) {
                continue;
            }

            if comparable.contains(&property.as_str()) {
                continue;
            }

            let from_property_value = from_object
                .get(property)
                .unwrap_or(&serde_json::Value::Null);

            if from_property_value != to_property_value {
                divergences.push(property.clone());
            }
        }
    }

    divergences
}

fn topologically_sort_creations<T: UniversalEntityLike>(
    metadata_name: MetadataName,
    creations: &UniversalFlatEntityMaps<T>,
) -> Vec<String> {
    let self_reference_property = match metadata_name {
        MetadataName::FieldMetadata => Some("relationTargetFieldMetadataUniversalIdentifier"),
        MetadataName::NavigationMenuItem => Some("parentUniversalIdentifier"),
        _ => None,
    };

    let universal_identifiers: Vec<String> =
        creations.by_universal_identifier.keys().cloned().collect();

    let self_reference_property = match self_reference_property {
        Some(property) => property,
        None => return universal_identifiers,
    };

    let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();

    for (universal_identifier, entity) in &creations.by_universal_identifier {
        let payload = entity.to_diff_value();
        let target = payload
            .get(self_reference_property)
            .and_then(serde_json::Value::as_str);

        if let Some(target) = target {
            if creations.by_universal_identifier.contains_key(target) {
                dependencies
                    .entry(target.to_string())
                    .or_default()
                    .push(universal_identifier.clone());
            }
        }
    }

    let mut ordered: Vec<String> = Vec::with_capacity(universal_identifiers.len());
    let mut placed: HashSet<String> = HashSet::new();

    while ordered.len() < universal_identifiers.len() {
        let mut progressed = false;

        for universal_identifier in &universal_identifiers {
            if placed.contains(universal_identifier) {
                continue;
            }

            let ready = dependencies
                .get(universal_identifier)
                .map(|upstream| {
                    upstream
                        .iter()
                        .all(|dependency| placed.contains(dependency))
                })
                .unwrap_or(true);

            if ready {
                ordered.push(universal_identifier.clone());
                placed.insert(universal_identifier.clone());
                progressed = true;
            }
        }

        if !progressed {
            for universal_identifier in &universal_identifiers {
                if !placed.contains(universal_identifier) {
                    ordered.push(universal_identifier.clone());
                }
            }

            break;
        }
    }

    ordered
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceMigrationBuildResult {
    Fail(OrchestratorFailureReport),
    Success(WorkspaceMigration),
}

pub fn build_workspace_migration(
    build_options: BuildOptions,
    from_to_maps: FromToAllMaps,
) -> WorkspaceMigrationBuildResult {
    let mut optimistic_maps = UniversalWorkspaceMaps {
        object_metadata_maps: from_to_maps.object_metadata.from.clone(),
        field_metadata_maps: from_to_maps.field_metadata.from.clone(),
        index_metadata_maps: from_to_maps.index_metadata.from.clone(),
        field_metadata_option_maps: from_to_maps.field_metadata_option.from.clone(),
        search_field_metadata_maps: from_to_maps.search_field_metadata.from.clone(),
    };

    let mut failure_report = OrchestratorFailureReport::default();
    let mut actions_report = OrchestratorActionsReport::default();

    {
        let outcome = validate_and_build(
            &object::ObjectActionsBuilder,
            &build_options,
            &mut optimistic_maps,
            &from_to_maps.object_metadata.from,
            &from_to_maps.object_metadata.to,
        );

        merge_outcome(
            &mut failure_report,
            &mut actions_report,
            MetadataName::ObjectMetadata,
            outcome,
        );
    }

    {
        let outcome = validate_and_build(
            &field::FieldActionsBuilder,
            &build_options,
            &mut optimistic_maps,
            &from_to_maps.field_metadata.from,
            &from_to_maps.field_metadata.to,
        );

        merge_outcome(
            &mut failure_report,
            &mut actions_report,
            MetadataName::FieldMetadata,
            outcome,
        );
    }

    {
        let outcome = validate_and_build(
            &field_option::FieldMetadataOptionActionsBuilder,
            &build_options,
            &mut optimistic_maps,
            &from_to_maps.field_metadata_option.from,
            &from_to_maps.field_metadata_option.to,
        );

        merge_outcome(
            &mut failure_report,
            &mut actions_report,
            MetadataName::FieldMetadataOption,
            outcome,
        );
    }

    {
        let outcome = validate_and_build(
            &index::IndexActionsBuilder,
            &build_options,
            &mut optimistic_maps,
            &from_to_maps.index_metadata.from,
            &from_to_maps.index_metadata.to,
        );

        merge_outcome(
            &mut failure_report,
            &mut actions_report,
            MetadataName::Index,
            outcome,
        );
    }

    {
        let outcome = validate_and_build(
            &search_field::SearchFieldMetadataActionsBuilder,
            &build_options,
            &mut optimistic_maps,
            &from_to_maps.search_field_metadata.from,
            &from_to_maps.search_field_metadata.to,
        );

        merge_outcome(
            &mut failure_report,
            &mut actions_report,
            MetadataName::SearchFieldMetadata,
            outcome,
        );
    }

    let cross_entity_failures = cross_entity_transversal_validation(
        &optimistic_maps,
        &actions_report,
        &from_to_maps,
        &build_options,
    );

    failure_report.merge(cross_entity_failures);

    if failure_report.total_errors() > 0 {
        return WorkspaceMigrationBuildResult::Fail(failure_report);
    }

    let search_vector_universal_identifiers_to_rebuild =
        compute_search_vector_rebuild_target_universal_identifiers(
            &actions_report,
            &from_to_maps.search_field_metadata.from,
            &optimistic_maps.search_field_metadata_maps,
            &optimistic_maps.field_metadata_maps,
        );

    let aggregated = aggregate_orchestrator_actions_report(
        actions_report,
        &optimistic_maps.field_metadata_maps,
        &search_vector_universal_identifiers_to_rebuild,
    );

    let actions = compute_ordered_migration_actions(&aggregated);

    WorkspaceMigrationBuildResult::Success(WorkspaceMigration {
        actions,
        application_universal_identifier: build_options.application_universal_identifier.clone(),
    })
}

fn merge_outcome(
    failure_report: &mut OrchestratorFailureReport,
    actions_report: &mut OrchestratorActionsReport,
    metadata_name: MetadataName,
    outcome: ValidateAndBuildOutcome,
) {
    match outcome {
        ValidateAndBuildOutcome::Success {
            create,
            update,
            delete,
        } => {
            let bucket = actions_report.actions_for_mut(metadata_name);
            bucket.create.extend(create);
            bucket.update.extend(update);
            bucket.delete.extend(delete);
        }
        ValidateAndBuildOutcome::Fail(failures) => {
            failure_report.push_all(metadata_name, failures);
        }
    }
}

pub fn compute_from_to_maps_from_matrix(matrix: &FlatEntityOperationsMatrix) -> FromToAllMaps {
    let mut from_to = FromToAllMaps::default();

    for metadata_name in MetadataName::all() {
        let operations = match matrix.by_metadata_name.get(metadata_name) {
            Some(operations) => operations,
            None => continue,
        };

        match metadata_name {
            MetadataName::ObjectMetadata => {
                apply_typed_operations(&mut from_to.object_metadata, operations)
            }
            MetadataName::FieldMetadata => {
                apply_typed_operations(&mut from_to.field_metadata, operations)
            }
            MetadataName::Index => apply_typed_operations(&mut from_to.index_metadata, operations),
            MetadataName::FieldMetadataOption => {
                apply_typed_operations(&mut from_to.field_metadata_option, operations)
            }
            MetadataName::SearchFieldMetadata => {
                apply_typed_operations(&mut from_to.search_field_metadata, operations)
            }
            _ => {}
        }
    }

    from_to
}

fn apply_typed_operations<E: UniversalEntityLike>(
    from_to: &mut crate::metadata::flat::FromToMaps<UniversalFlatEntityMaps<E>>,
    operations: &FlatEntityToCreateDeleteUpdate,
) {
    for entity in &operations.to_delete {
        from_to
            .from
            .delete_by_universal_identifier(entity.universal_identifier());
    }

    for entity in operations
        .to_create
        .iter()
        .chain(operations.to_update.iter())
    {
        let value = match entity.to_value() {
            Ok(value) => value,
            Err(_) => continue,
        };

        let typed: E = match serde_json::from_value(value) {
            Ok(typed) => typed,
            Err(_) => continue,
        };

        from_to
            .to
            .add(entity.universal_identifier().to_string(), typed);
    }
}
