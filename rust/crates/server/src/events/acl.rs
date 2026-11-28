#![allow(dead_code)]

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

use sqlx::PgPool;
use uuid::Uuid;

use super::constants::is_tool_permission_flag;
use super::gate::PermissionFlagAcl;

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionAclError {
    pub message: String,
}

impl PermissionAclError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PermissionAclError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "permission ACL load failed: {}", self.message)
    }
}

impl std::error::Error for PermissionAclError {}

impl From<sqlx::Error> for PermissionAclError {
    fn from(error: sqlx::Error) -> Self {
        Self::new(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleGrantRow {
    pub role_id: Uuid,
    pub can_update_all_settings: bool,
    pub can_access_all_tools: bool,
    pub flag_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleTargetRow {
    pub role_id: Uuid,
    pub user_workspace_id: Uuid,
}

pub fn grant_matrix(
    role_rows: Vec<RoleGrantRow>,
    target_rows: Vec<RoleTargetRow>,
    flags: &[&str],
) -> PermissionFlagAcl {
    let targets_by_role =
        target_rows
            .into_iter()
            .fold(HashMap::<Uuid, Vec<Uuid>>::new(), |mut acc, row| {
                acc.entry(row.role_id)
                    .or_default()
                    .push(row.user_workspace_id);
                acc
            });

    let mut allowed: HashMap<String, HashSet<Uuid>> = HashMap::new();
    for flag in flags {
        let mut user_workspace_ids: HashSet<Uuid> = HashSet::new();
        for row in &role_rows {
            if !role_has_permission_flag(row, flag) {
                continue;
            }
            if let Some(targets) = targets_by_role.get(&row.role_id) {
                user_workspace_ids.extend(targets.iter().copied());
            }
        }
        allowed.insert((*flag).to_string(), user_workspace_ids);
    }

    PermissionFlagAcl::new(allowed)
}

fn role_has_permission_flag(row: &RoleGrantRow, flag: &str) -> bool {
    if row.flag_key.as_deref() == Some(flag) {
        return true;
    }
    if is_tool_permission_flag(flag) {
        return row.can_access_all_tools;
    }
    row.can_update_all_settings
}

pub async fn load_permission_flag_acl(
    pool: &PgPool,
    workspace_id: Uuid,
    flags: &[&str],
) -> Result<PermissionFlagAcl, PermissionAclError> {
    let role_rows = sqlx::query_as::<_, (Uuid, bool, bool, Option<String>)>(
        "SELECT r.id, r.\"canUpdateAllSettings\", r.\"canAccessAllTools\", pf.key \
         FROM core.role r \
         LEFT JOIN core.\"rolePermissionFlag\" rpf ON rpf.\"roleId\" = r.id \
         LEFT JOIN core.\"permissionFlag\" pf ON pf.id = rpf.\"permissionFlagId\" \
         WHERE r.\"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    let target_rows = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT rt.\"roleId\", rt.\"userWorkspaceId\" FROM core.\"roleTarget\" rt \
         WHERE rt.\"workspaceId\" = $1 AND rt.\"userWorkspaceId\" IS NOT NULL",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    let role_rows = role_rows
        .into_iter()
        .map(
            |(role_id, can_update_all_settings, can_access_all_tools, flag_key)| RoleGrantRow {
                role_id,
                can_update_all_settings,
                can_access_all_tools,
                flag_key,
            },
        )
        .collect::<Vec<_>>();
    let target_rows = target_rows
        .into_iter()
        .map(|(role_id, user_workspace_id)| RoleTargetRow {
            role_id,
            user_workspace_id,
        })
        .collect::<Vec<_>>();

    Ok(grant_matrix(role_rows, target_rows, flags))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(role_id: Uuid, can_update_all_settings: bool, flag_key: Option<&str>) -> RoleGrantRow {
        RoleGrantRow {
            role_id,
            can_update_all_settings,
            can_access_all_tools: false,
            flag_key: flag_key.map(str::to_string),
        }
    }

    #[test]
    fn non_tool_flags_are_granted_role_wide_by_update_all_settings() {
        let role_id = Uuid::new_v4();
        let member = Uuid::new_v4();
        let acl = grant_matrix(
            vec![role(role_id, true, None)],
            vec![RoleTargetRow {
                role_id,
                user_workspace_id: member,
            }],
            &["WORKFLOWS"],
        );
        assert!(acl.allows("WORKFLOWS", member));
        assert!(!acl.allows("SEE_AUDIT_LOGS", member));
    }

    #[test]
    fn tool_flags_require_access_all_tools() {
        let role_id = Uuid::new_v4();
        let member = Uuid::new_v4();
        let target = RoleTargetRow {
            role_id,
            user_workspace_id: member,
        };
        let settings_only = grant_matrix(
            vec![RoleGrantRow {
                role_id,
                can_update_all_settings: true,
                can_access_all_tools: false,
                flag_key: None,
            }],
            vec![target.clone()],
            &["VIEWS"],
        );
        assert!(!settings_only.allows("VIEWS", member));

        let tool_capable = grant_matrix(
            vec![RoleGrantRow {
                role_id,
                can_update_all_settings: false,
                can_access_all_tools: true,
                flag_key: None,
            }],
            vec![target],
            &["VIEWS"],
        );
        assert!(tool_capable.allows("VIEWS", member));
    }

    #[test]
    fn explicit_flag_rows_grant_regardless_of_role_wide_columns() {
        let role_id = Uuid::new_v4();
        let member = Uuid::new_v4();
        let acl = grant_matrix(
            vec![role(role_id, false, Some("WORKFLOWS"))],
            vec![RoleTargetRow {
                role_id,
                user_workspace_id: member,
            }],
            &["WORKFLOWS"],
        );
        assert!(acl.allows("WORKFLOWS", member));
    }

    #[test]
    fn targets_are_unioned_across_granted_roles_and_missing_targets_are_empty() {
        let wide = Uuid::new_v4();
        let flagged = Uuid::new_v4();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let third = Uuid::new_v4();
        let acl = grant_matrix(
            vec![
                role(wide, true, None),
                RoleGrantRow {
                    role_id: flagged,
                    can_update_all_settings: false,
                    can_access_all_tools: false,
                    flag_key: Some(String::from("SEE_AUDIT_LOGS")),
                },
            ],
            vec![
                RoleTargetRow {
                    role_id: wide,
                    user_workspace_id: first,
                },
                RoleTargetRow {
                    role_id: wide,
                    user_workspace_id: second,
                },
                RoleTargetRow {
                    role_id: flagged,
                    user_workspace_id: third,
                },
            ],
            &["WORKFLOWS", "SEE_AUDIT_LOGS", "VIEWS"],
        );
        assert!(acl.allows("WORKFLOWS", first) && acl.allows("WORKFLOWS", second));
        assert!(!acl.allows("WORKFLOWS", third));
        assert!(!acl.allows("VIEWS", first));
        assert!(acl.allows("SEE_AUDIT_LOGS", third));
        assert!(acl.allows("SEE_AUDIT_LOGS", first));
        let empty = grant_matrix(vec![role(wide, true, None)], vec![], &["WORKFLOWS"]);
        assert!(!empty.allows("WORKFLOWS", first));
        assert!(!empty.is_empty());
    }
}
