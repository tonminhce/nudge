#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Mutex;

use uuid::Uuid;

use crate::metadata::error::MetadataResult;
use crate::metadata::flat::load_workspace_flat_maps;
use crate::metadata::flat::FlatWorkspaceMaps;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalWorkspaceMaps;

pub struct WorkspaceFlatMapsCache {
    maps_by_workspace: Mutex<HashMap<Uuid, FlatWorkspaceMaps>>,
}

impl Default for WorkspaceFlatMapsCache {
    fn default() -> Self {
        Self {
            maps_by_workspace: Mutex::new(HashMap::new()),
        }
    }
}

impl WorkspaceFlatMapsCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_or_recompute(
        &self,
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
    ) -> MetadataResult<FlatWorkspaceMaps> {
        if let Some(maps) = self.peek(&workspace_id) {
            return Ok(maps);
        }

        let maps = load_workspace_flat_maps(pool, workspace_id).await?;

        self.store(workspace_id, maps.clone());

        Ok(maps)
    }

    pub fn peek(&self, workspace_id: &Uuid) -> Option<FlatWorkspaceMaps> {
        self.maps_by_workspace
            .lock()
            .ok()?
            .get(workspace_id)
            .cloned()
    }

    pub fn store(&self, workspace_id: Uuid, maps: FlatWorkspaceMaps) {
        if let Ok(mut guard) = self.maps_by_workspace.lock() {
            guard.insert(workspace_id, maps);
        }
    }

    pub fn invalidate(&self, workspace_id: &Uuid) -> bool {
        match self.maps_by_workspace.lock() {
            Ok(mut guard) => guard.remove(workspace_id).is_some(),
            Err(_) => false,
        }
    }

    pub fn invalidate_all(&self) -> usize {
        match self.maps_by_workspace.lock() {
            Ok(mut guard) => {
                let count = guard.len();
                guard.clear();
                count
            }
            Err(_) => 0,
        }
    }
}

pub fn to_universal_workspace_maps(flat_maps: &FlatWorkspaceMaps) -> UniversalWorkspaceMaps {
    let mut object_maps: UniversalFlatEntityMaps<
        crate::metadata::flat::UniversalFlatObjectMetadata,
    > = UniversalFlatEntityMaps::default();

    for (universal_identifier, object) in &flat_maps.object_metadata_maps.by_universal_identifier {
        object_maps.add(universal_identifier.clone(), object.clone().universal());
    }

    let mut field_maps: UniversalFlatEntityMaps<crate::metadata::flat::UniversalFlatFieldMetadata> =
        UniversalFlatEntityMaps::default();

    for (universal_identifier, field) in &flat_maps.field_metadata_maps.by_universal_identifier {
        field_maps.add(universal_identifier.clone(), field.clone().universal());
    }

    let mut index_maps: UniversalFlatEntityMaps<crate::metadata::flat::UniversalFlatIndexMetadata> =
        UniversalFlatEntityMaps::default();

    for (universal_identifier, index) in &flat_maps.index_metadata_maps.by_universal_identifier {
        index_maps.add(universal_identifier.clone(), universal_index(index));
    }

    let mut option_maps: UniversalFlatEntityMaps<
        crate::metadata::flat::UniversalFlatFieldMetadataOption,
    > = UniversalFlatEntityMaps::default();

    for (universal_identifier, option) in flat_maps
        .field_metadata_option_maps
        .by_universal_identifier
        .iter()
    {
        option_maps.add(universal_identifier.clone(), option.clone().universal());
    }

    let mut search_field_maps: UniversalFlatEntityMaps<
        crate::metadata::flat::UniversalFlatSearchFieldMetadata,
    > = UniversalFlatEntityMaps::default();

    for (universal_identifier, search_field) in flat_maps
        .search_field_metadata_maps
        .by_universal_identifier
        .iter()
    {
        search_field_maps.add(
            universal_identifier.clone(),
            search_field.clone().universal(),
        );
    }

    UniversalWorkspaceMaps {
        object_metadata_maps: object_maps,
        field_metadata_maps: field_maps,
        index_metadata_maps: index_maps,
        field_metadata_option_maps: option_maps,
        search_field_metadata_maps: search_field_maps,
    }
}

fn universal_index(
    index: &crate::metadata::flat::FlatIndexMetadata,
) -> crate::metadata::flat::UniversalFlatIndexMetadata {
    crate::metadata::flat::UniversalFlatIndexMetadata {
        universal_identifier: index.universal_identifier.clone(),
        application_universal_identifier: String::new(),
        name: index.name.clone(),
        object_metadata_universal_identifier: index.object_metadata_universal_identifier.clone(),
        is_unique: index.is_unique,
        index_where_clause: index.index_where_clause.clone(),
        index_type: index.index_type,
        is_system_side_effect: index.is_system_side_effect,
        created_at: index.created_at,
        updated_at: index.updated_at,
        universal_flat_index_field_metadatas: index.universal_flat_index_field_metadatas.clone(),
    }
}

pub fn merge_flat_action_into_maps(
    maps: &mut FlatWorkspaceMaps,
    metadata_name: crate::metadata::actions::MetadataName,
    operation: crate::metadata::actions::WorkspaceMigrationActionType,
    universal_identifier: &str,
) -> bool {
    use crate::metadata::actions::MetadataName as Name;
    use crate::metadata::actions::WorkspaceMigrationActionType as Operation;

    match (metadata_name, operation) {
        (Name::ObjectMetadata, Operation::Delete) => maps
            .object_metadata_maps
            .delete_by_universal_identifier(universal_identifier)
            .is_some(),
        (Name::FieldMetadata, Operation::Delete) => maps
            .field_metadata_maps
            .delete_by_universal_identifier(universal_identifier)
            .is_some(),
        (Name::Index, Operation::Delete) => maps
            .index_metadata_maps
            .delete_by_universal_identifier(universal_identifier)
            .is_some(),
        (Name::FieldMetadataOption, Operation::Delete) => maps
            .field_metadata_option_maps
            .delete_by_universal_identifier(universal_identifier)
            .is_some(),
        (Name::SearchFieldMetadata, Operation::Delete) => maps
            .search_field_metadata_maps
            .delete_by_universal_identifier(universal_identifier)
            .is_some(),
        _ => false,
    }
}

pub fn metadata_version_key(workspace_id: &Uuid) -> String {
    format!("workspace-metadata-version:{workspace_id}")
}

pub async fn invalidate_workspace_maps(state: &crate::state::AppState, workspace_id: Uuid) {
    state.metadata_cache.invalidate(&workspace_id);
    state.permission_cache.invalidate_workspace(workspace_id);

    let Ok(mut connection) = state.redis.get_multiplexed_async_connection().await else {
        return;
    };

    let _ = redis::cmd("DEL")
        .arg(metadata_version_key(&workspace_id))
        .query_async::<()>(&mut connection)
        .await;
}

pub async fn rebuild_access_groups_after_invalidation(
    state: &crate::state::AppState,
    workspace_id: Uuid,
) {
    match crate::permissions::access_group::rebuild_company_access_groups(
        &state.core_pool,
        workspace_id,
    )
    .await
    {
        Ok(crate::permissions::access_group::RebuildOutcome::Materialized(outcome)) => {
            tracing::debug!(
                workspace_id = %workspace_id,
                group_count = outcome.group_count,
                updated_record_count = outcome.updated_record_count,
                "company access groups materialized"
            );
        }
        Ok(_) => {}
        Err(error) => {
            tracing::error!(
                workspace_id = %workspace_id,
                error = %error,
                "company access group rebuild failed; reads will use live RLS"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::merge_flat_action_into_maps;
    use super::metadata_version_key;
    use super::to_universal_workspace_maps;
    use super::WorkspaceFlatMapsCache;
    use crate::metadata::actions::MetadataName;
    use crate::metadata::actions::WorkspaceMigrationActionType;
    use crate::metadata::flat::FlatFieldMetadata;
    use crate::metadata::flat::FlatWorkspaceMaps;

    fn field(id: Uuid, universal_identifier: &str) -> FlatFieldMetadata {
        FlatFieldMetadata {
            id,
            universal_identifier: universal_identifier.to_string(),
            ..FlatFieldMetadata::default()
        }
    }

    #[test]
    fn cache_stores_and_invalidates_per_workspace() {
        let cache = WorkspaceFlatMapsCache::new();
        let workspace_id = Uuid::new_v4();

        assert!(cache.peek(&workspace_id).is_none());
        assert!(!cache.invalidate(&workspace_id));

        cache.store(workspace_id, FlatWorkspaceMaps::default());
        assert!(cache.peek(&workspace_id).is_some());
        assert!(cache.invalidate(&workspace_id));
        assert!(cache.peek(&workspace_id).is_none());

        cache.store(workspace_id, FlatWorkspaceMaps::default());
        assert_eq!(cache.invalidate_all(), 1);
        assert!(cache.peek(&workspace_id).is_none());
    }

    #[test]
    fn merge_deletes_flat_entities_by_name() {
        let mut maps = FlatWorkspaceMaps::default();
        let field_id = Uuid::new_v4();

        maps.field_metadata_maps.add(
            field_id.to_string(),
            field(field_id, "20202020-4d99-46a1-9b8c-33e6c9b0d5c6"),
            "20202020-4d99-46a1-9b8c-33e6c9b0d5c6",
            Uuid::new_v4().to_string(),
        );

        assert!(merge_flat_action_into_maps(
            &mut maps,
            MetadataName::FieldMetadata,
            WorkspaceMigrationActionType::Delete,
            "20202020-4d99-46a1-9b8c-33e6c9b0d5c6",
        ));
        assert!(!merge_flat_action_into_maps(
            &mut maps,
            MetadataName::FieldMetadata,
            WorkspaceMigrationActionType::Delete,
            "20202020-4d99-46a1-9b8c-33e6c9b0d5c6",
        ));
        assert!(!merge_flat_action_into_maps(
            &mut maps,
            MetadataName::ObjectMetadata,
            WorkspaceMigrationActionType::Update,
            "20202020-4d99-46a1-9b8c-33e6c9b0d5c6",
        ));
    }

    #[test]
    fn flat_maps_convert_to_universal_maps() {
        let mut maps = FlatWorkspaceMaps::default();
        let field_id = Uuid::new_v4();

        maps.field_metadata_maps.add(
            field_id.to_string(),
            field(field_id, "20202020-4d99-46a1-9b8c-33e6c9b0d5c6"),
            "20202020-4d99-46a1-9b8c-33e6c9b0d5c6",
            Uuid::new_v4().to_string(),
        );

        let universal = to_universal_workspace_maps(&maps);

        assert!(universal
            .field_metadata_maps
            .find_by_universal_identifier("20202020-4d99-46a1-9b8c-33e6c9b0d5c6")
            .is_some());
        assert!(universal
            .object_metadata_maps
            .by_universal_identifier
            .is_empty());
    }

    #[test]
    fn metadata_version_key_is_stable() {
        let workspace_id = Uuid::nil();

        assert_eq!(
            metadata_version_key(&workspace_id),
            format!("workspace-metadata-version:{workspace_id}")
        );
    }
}
