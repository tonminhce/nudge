#![allow(dead_code)]

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::OrchestratorActionsReport;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::WorkspaceMigrationActionType;

type Bucket = (MetadataName, WorkspaceMigrationActionType);

const ACTION_EMISSION_ORDER: &[Bucket] = &[
    (
        MetadataName::SearchFieldMetadata,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::SearchFieldMetadata,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::SearchFieldMetadata,
        WorkspaceMigrationActionType::Update,
    ),
    (MetadataName::Index, WorkspaceMigrationActionType::Delete),
    (
        MetadataName::FieldMetadataOption,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::FieldMetadata,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ObjectMetadata,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ObjectMetadata,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ObjectMetadata,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::FieldMetadata,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::FieldMetadata,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::FieldMetadataOption,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::FieldMetadataOption,
        WorkspaceMigrationActionType::Update,
    ),
    (MetadataName::Index, WorkspaceMigrationActionType::Create),
    (MetadataName::Index, WorkspaceMigrationActionType::Update),
    (MetadataName::View, WorkspaceMigrationActionType::Create),
    (MetadataName::View, WorkspaceMigrationActionType::Update),
    (
        MetadataName::ViewField,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ViewFieldGroup,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ViewFieldGroup,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ViewFieldGroup,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::ViewField,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ViewField,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::ViewFilterGroup,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ViewFilterGroup,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::ViewFilter,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ViewFilter,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ViewFilter,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::ViewFilterGroup,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ViewGroup,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ViewGroup,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ViewGroup,
        WorkspaceMigrationActionType::Update,
    ),
    (MetadataName::ViewSort, WorkspaceMigrationActionType::Delete),
    (MetadataName::ViewSort, WorkspaceMigrationActionType::Create),
    (MetadataName::ViewSort, WorkspaceMigrationActionType::Update),
    (MetadataName::View, WorkspaceMigrationActionType::Delete),
    (
        MetadataName::WorkflowVersion,
        WorkspaceMigrationActionType::Delete,
    ),
    (MetadataName::Workflow, WorkspaceMigrationActionType::Delete),
    (MetadataName::Workflow, WorkspaceMigrationActionType::Create),
    (MetadataName::Workflow, WorkspaceMigrationActionType::Update),
    (
        MetadataName::WorkflowVersion,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::WorkflowVersion,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::LogicFunction,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::LogicFunction,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::LogicFunction,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::AgentChatChannelRole,
        WorkspaceMigrationActionType::Delete,
    ),
    (MetadataName::Role, WorkspaceMigrationActionType::Delete),
    (MetadataName::Role, WorkspaceMigrationActionType::Create),
    (MetadataName::Role, WorkspaceMigrationActionType::Update),
    (
        MetadataName::RoleTarget,
        WorkspaceMigrationActionType::Delete,
    ),
    (MetadataName::Agent, WorkspaceMigrationActionType::Delete),
    (MetadataName::Agent, WorkspaceMigrationActionType::Create),
    (MetadataName::Agent, WorkspaceMigrationActionType::Update),
    (MetadataName::Webhook, WorkspaceMigrationActionType::Delete),
    (MetadataName::Webhook, WorkspaceMigrationActionType::Create),
    (MetadataName::Webhook, WorkspaceMigrationActionType::Update),
    (
        MetadataName::RoleTarget,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::RoleTarget,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::ObjectPermission,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ObjectPermission,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ObjectPermission,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::RowLevelPermissionRule,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::RowLevelPermissionRule,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::RolePermissionFlag,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::PermissionFlag,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::PermissionFlag,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::RolePermissionFlag,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::PermissionFlag,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::RolePermissionFlag,
        WorkspaceMigrationActionType::Update,
    ),
    (MetadataName::Skill, WorkspaceMigrationActionType::Delete),
    (MetadataName::Skill, WorkspaceMigrationActionType::Create),
    (MetadataName::Skill, WorkspaceMigrationActionType::Update),
    (
        MetadataName::AgentChatChannel,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::AgentChatChannel,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::AgentChatChannel,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::AgentChatChannelRole,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::AgentChatChannelRole,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::CommandMenuItem,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::SettingsMenuItem,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::FrontComponent,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::FrontComponent,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::PageLayout,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::CommandMenuItem,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::CommandMenuItem,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::SettingsMenuItem,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::SettingsMenuItem,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::FrontComponent,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::PageLayoutTab,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::PageLayoutTab,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::PageLayout,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::PageLayoutWidget,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::PageLayoutWidget,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::PageLayoutWidget,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::PageLayoutTab,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::PageLayout,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::NavigationMenuItem,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::NavigationMenuItem,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::NavigationMenuItem,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::RowLevelPermissionPredicateGroup,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::RowLevelPermissionPredicateGroup,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::FieldPermission,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::FieldPermission,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::FieldPermission,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::RowLevelPermissionPredicate,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::RowLevelPermissionPredicate,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::RowLevelPermissionPredicate,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::RowLevelPermissionPredicateGroup,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::RowLevelPermissionRule,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::McpServer,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ApplicationVariable,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ApplicationVariable,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ApplicationVariable,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::ConnectionProvider,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ConnectionProvider,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ConnectionProvider,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::MessagingProvider,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::MessagingProvider,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::MessagingProvider,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::McpServer,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::McpServer,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::TimelineActivityType,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::TimelineActivityType,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::TimelineActivityType,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::RelatedRecordPath,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::ValidationRule,
        WorkspaceMigrationActionType::Delete,
    ),
    (
        MetadataName::RelatedRecordPath,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::ValidationRule,
        WorkspaceMigrationActionType::Create,
    ),
    (
        MetadataName::RelatedRecordPath,
        WorkspaceMigrationActionType::Update,
    ),
    (
        MetadataName::ValidationRule,
        WorkspaceMigrationActionType::Update,
    ),
];

pub fn compute_ordered_migration_actions(
    aggregated_actions_report: &OrchestratorActionsReport,
) -> Vec<UniversalAction> {
    let mut ordered: Vec<UniversalAction> = Vec::new();

    for (metadata_name, action_type) in ACTION_EMISSION_ORDER {
        let bucket = aggregated_actions_report.actions_for(*metadata_name);

        let matching_actions = match action_type {
            WorkspaceMigrationActionType::Create => &bucket.create,
            WorkspaceMigrationActionType::Update => &bucket.update,
            WorkspaceMigrationActionType::Delete => &bucket.delete,
        };

        ordered.extend(matching_actions.iter().cloned());
    }

    ordered
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::compute_ordered_migration_actions;
    use crate::metadata::actions::ActionsByOperation;
    use crate::metadata::actions::MetadataName;
    use crate::metadata::actions::OrchestratorActionsReport;
    use crate::metadata::actions::UniversalAction;
    use crate::metadata::actions::UniversalCreateAction;
    use crate::metadata::actions::UniversalDeleteAction;
    use crate::metadata::actions::UniversalFlatEntity;
    use crate::metadata::actions::UniversalUpdateAction;
    use crate::metadata::flat::UniversalFlatFieldMetadata;
    use crate::metadata::flat::UniversalFlatObjectMetadata;
    use crate::metadata::flat::UniversalFlatSearchFieldMetadata;

    fn create_object(universal_identifier: &str) -> UniversalAction {
        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::ObjectMetadata,
            flat_entity: Some(UniversalFlatEntity::ObjectMetadata(Box::new(
                UniversalFlatObjectMetadata {
                    universal_identifier: universal_identifier.to_string(),
                    name_singular: format!("object_{universal_identifier}"),
                    ..UniversalFlatObjectMetadata::default()
                },
            ))),
            ..UniversalCreateAction::default()
        })
    }

    fn create_field(universal_identifier: &str) -> UniversalAction {
        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::FieldMetadata,
            flat_entity: Some(UniversalFlatEntity::FieldMetadata(Box::new(
                UniversalFlatFieldMetadata {
                    universal_identifier: universal_identifier.to_string(),
                    ..UniversalFlatFieldMetadata::default()
                },
            ))),
            ..UniversalCreateAction::default()
        })
    }

    fn delete_field(universal_identifier: &str) -> UniversalAction {
        UniversalAction::Delete(UniversalDeleteAction {
            metadata_name: MetadataName::FieldMetadata,
            universal_identifier: universal_identifier.to_string(),
            flat_entity: None,
        })
    }

    fn delete_object(universal_identifier: &str) -> UniversalAction {
        UniversalAction::Delete(UniversalDeleteAction {
            metadata_name: MetadataName::ObjectMetadata,
            universal_identifier: universal_identifier.to_string(),
            flat_entity: None,
        })
    }

    fn update_search_field(universal_identifier: &str) -> UniversalAction {
        UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::SearchFieldMetadata,
            universal_identifier: universal_identifier.to_string(),
            update: json!({ "position": 2.0 }),
            ..UniversalUpdateAction::default()
        })
    }

    fn search_field_create(universal_identifier: &str) -> UniversalAction {
        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::SearchFieldMetadata,
            flat_entity: Some(UniversalFlatEntity::SearchFieldMetadata(Box::new(
                UniversalFlatSearchFieldMetadata {
                    universal_identifier: universal_identifier.to_string(),
                    ..UniversalFlatSearchFieldMetadata::default()
                },
            ))),
            ..UniversalCreateAction::default()
        })
    }

    #[test]
    fn object_deletes_run_before_field_deletes() {
        let mut report = OrchestratorActionsReport::default();

        let field_bucket = report.actions_for_mut(MetadataName::FieldMetadata);
        field_bucket.delete.push(delete_field("field-1"));

        let object_bucket = report.actions_for_mut(MetadataName::ObjectMetadata);
        object_bucket.delete.push(delete_object("object-1"));

        let ordered = compute_ordered_migration_actions(&report);

        let field_delete_position = ordered
            .iter()
            .position(|action| {
                action.action_type()
                    == crate::metadata::actions::WorkspaceMigrationActionType::Delete
                    && action.metadata_name() == MetadataName::FieldMetadata
            })
            .unwrap();
        let object_delete_position = ordered
            .iter()
            .position(|action| {
                action.action_type()
                    == crate::metadata::actions::WorkspaceMigrationActionType::Delete
                    && action.metadata_name() == MetadataName::ObjectMetadata
            })
            .unwrap();

        assert!(
            field_delete_position < object_delete_position,
            "field deletes must run before object deletes so the object's DROP TABLE CASCADE lands last"
        );
    }

    #[test]
    fn object_creates_run_before_field_creates() {
        let mut report = OrchestratorActionsReport::default();

        let field_bucket = report.actions_for_mut(MetadataName::FieldMetadata);
        field_bucket.create.push(create_field("field-1"));

        let object_bucket = report.actions_for_mut(MetadataName::ObjectMetadata);
        object_bucket.create.push(create_object("object-1"));

        let ordered = compute_ordered_migration_actions(&report);

        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].metadata_name(), MetadataName::ObjectMetadata);
        assert_eq!(ordered[1].metadata_name(), MetadataName::FieldMetadata);
    }

    #[test]
    fn search_field_actions_run_first_and_index_creates_run_last_of_schema_block() {
        let mut report = OrchestratorActionsReport::default();

        report
            .actions_for_mut(MetadataName::SearchFieldMetadata)
            .create
            .push(search_field_create("search-1"));

        let field_bucket = report.actions_for_mut(MetadataName::FieldMetadata);
        field_bucket.create.push(create_field("field-1"));

        let object_bucket = report.actions_for_mut(MetadataName::ObjectMetadata);
        object_bucket.create.push(create_object("object-1"));

        let ordered = compute_ordered_migration_actions(&report);

        assert_eq!(ordered.len(), 3);
        assert_eq!(
            ordered[0].metadata_name(),
            MetadataName::SearchFieldMetadata
        );
        assert_eq!(ordered[1].metadata_name(), MetadataName::ObjectMetadata);
        assert_eq!(ordered[2].metadata_name(), MetadataName::FieldMetadata);
    }

    #[test]
    fn emission_order_matches_upstream_sequence() {
        let mut report = OrchestratorActionsReport::default();

        report
            .actions_for_mut(MetadataName::SearchFieldMetadata)
            .update
            .push(update_search_field("search-update"));
        report
            .actions_for_mut(MetadataName::SearchFieldMetadata)
            .create
            .push(search_field_create("search-create"));

        let field_bucket = report.actions_for_mut(MetadataName::FieldMetadata);
        field_bucket.delete.push(delete_field("field-delete"));
        field_bucket.create.push(create_field("field-create"));

        let object_bucket = report.actions_for_mut(MetadataName::ObjectMetadata);
        object_bucket.delete.push(delete_object("object-delete"));
        object_bucket.create.push(create_object("object-create"));

        let ordered = compute_ordered_migration_actions(&report);

        let labels: Vec<String> = ordered
            .iter()
            .map(|action| {
                format!(
                    "{}:{}",
                    action.metadata_name().as_str(),
                    action.action_type().as_str()
                )
            })
            .collect();

        assert_eq!(
            labels,
            vec![
                "searchFieldMetadata:create",
                "searchFieldMetadata:update",
                "fieldMetadata:delete",
                "objectMetadata:delete",
                "objectMetadata:create",
                "fieldMetadata:create",
            ]
        );
    }

    #[test]
    fn view_filter_group_deletes_run_after_view_filter_updates_and_before_view_group() {
        let mut report = OrchestratorActionsReport::default();

        report
            .actions_for_mut(MetadataName::ViewFilter)
            .update
            .push(update_search_field("view-filter-update"));

        let group_bucket = report.actions_for_mut(MetadataName::ViewFilterGroup);
        group_bucket
            .delete
            .push(UniversalAction::Delete(UniversalDeleteAction {
                metadata_name: MetadataName::ViewFilterGroup,
                universal_identifier: "group-delete".to_string(),
                flat_entity: None,
            }));
        group_bucket
            .create
            .push(UniversalAction::Create(UniversalCreateAction {
                metadata_name: MetadataName::ViewFilterGroup,
                flat_entity: Some(UniversalFlatEntity::Other {
                    metadata_name: MetadataName::ViewFilterGroup,
                    entity: json!({ "universalIdentifier": "group-create" }),
                }),
                ..UniversalCreateAction::default()
            }));

        let ordered = compute_ordered_migration_actions(&report);

        let group_create_position = ordered.iter().position(|action| {
            action.metadata_name() == MetadataName::ViewFilterGroup
                && action.action_type()
                    == crate::metadata::actions::WorkspaceMigrationActionType::Create
        });
        let group_delete_position = ordered.iter().position(|action| {
            action.metadata_name() == MetadataName::ViewFilterGroup
                && action.action_type()
                    == crate::metadata::actions::WorkspaceMigrationActionType::Delete
        });

        assert!(group_create_position < group_delete_position);
    }

    #[test]
    fn empty_report_yields_no_actions() {
        let report = OrchestratorActionsReport::default();
        let ordered = compute_ordered_migration_actions(&report);

        assert!(ordered.is_empty());
        assert!(ActionsByOperation::default().is_empty());
    }
}
