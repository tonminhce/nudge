pub(crate) mod access_group;
mod context;
mod error;
mod kernel;
mod loader;
mod model;
mod restrict;
mod settings;
mod validate;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

use uuid::Uuid;

use model::WorkspacePermissionSet;

pub use error::PermissionError;
pub use model::ObjectRecordsOperationType;
pub use model::RecordsAccessSource;
pub use model::RoleRecordsPermissions;
pub use model::RowLevelRecordsPermissions;
pub use settings::current_user_workspace_permissions_for_auth;
pub use settings::records_permissions_for_auth;
pub use settings::user_workspace_permissions_for_auth;
pub use validate::assert_operation_allowed;
pub use validate::validate_operation_is_permitted_or_throw;

const PERMISSION_CACHE_CAPACITY: usize = 256;

pub struct PermissionCache {
    inner: Mutex<PermissionCacheState>,
}

#[derive(Default)]
struct PermissionCacheState {
    entries: HashMap<(Uuid, u64), WorkspacePermissionSet>,
    lru: VecDeque<(Uuid, u64)>,
    versions: HashMap<Uuid, u64>,
}

impl PermissionCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PermissionCacheState::default()),
        }
    }

    pub fn version(&self, workspace_id: Uuid) -> u64 {
        let state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        state.versions.get(&workspace_id).copied().unwrap_or(0)
    }

    pub fn get(&self, key: (Uuid, u64)) -> Option<WorkspacePermissionSet> {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let value = state.entries.get(&key).cloned();
        if value.is_some() {
            touch(&mut state, key);
        }
        value
    }

    pub fn insert(&self, key: (Uuid, u64), value: WorkspacePermissionSet) {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        touch(&mut state, key);
        state.entries.insert(key, value);
        while state.lru.len() > PERMISSION_CACHE_CAPACITY {
            if let Some(oldest) = state.lru.pop_front() {
                state.entries.remove(&oldest);
            }
        }
    }

    pub fn invalidate_workspace(&self, workspace_id: Uuid) {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        state.entries.retain(|(id, _), _| *id != workspace_id);
        state.lru.retain(|(id, _)| *id != workspace_id);
        let version = state.versions.entry(workspace_id).or_insert(0);
        *version = version.saturating_add(1);
    }
}

impl Default for PermissionCache {
    fn default() -> Self {
        Self::new()
    }
}

fn touch(state: &mut PermissionCacheState, key: (Uuid, u64)) {
    if let Some(index) = state.lru.iter().position(|entry| *entry == key) {
        state.lru.remove(index);
    }
    state.lru.push_back(key);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidation_bumps_the_workspace_cache_version() {
        let cache = PermissionCache::new();
        let workspace_id = Uuid::new_v4();
        assert_eq!(cache.version(workspace_id), 0);
        cache.invalidate_workspace(workspace_id);
        assert_eq!(cache.version(workspace_id), 1);
        cache.invalidate_workspace(workspace_id);
        assert_eq!(cache.version(workspace_id), 2);
    }

    #[test]
    fn cache_is_bounded_and_evicts_the_least_recently_used_entry() {
        let cache = PermissionCache::new();
        let workspace_id = Uuid::nil();
        for version in 0..=PERMISSION_CACHE_CAPACITY as u64 {
            cache.insert((workspace_id, version), WorkspacePermissionSet::default());
        }
        assert!(cache.get((workspace_id, 0)).is_none());
        assert!(cache
            .get((workspace_id, PERMISSION_CACHE_CAPACITY as u64))
            .is_some());
    }
}
