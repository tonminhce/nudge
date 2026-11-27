#![allow(dead_code)]

use std::collections::BTreeSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::jwt_service::TokenType;
use crate::auth::middleware::AuthContext;

use super::error::PermissionError;
use super::error::PermissionErrorCode;
use super::model::PermissionPrincipalKind;

#[derive(Debug, Clone)]
pub struct PermissionsContext {
    pub workspace_id: Option<Uuid>,
    pub principal_kind: PermissionPrincipalKind,
    pub user_id: Option<Uuid>,
    pub user_workspace_id: Option<Uuid>,
    pub workspace_member_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub application_id: Option<Uuid>,
    pub main_role_ids: Vec<Uuid>,
    pub limiting_role_ids: Vec<Uuid>,
    pub is_system: bool,
}

impl PermissionsContext {
    pub fn system(workspace_id: Option<Uuid>) -> Self {
        Self {
            workspace_id,
            principal_kind: PermissionPrincipalKind::System,
            user_id: None,
            user_workspace_id: None,
            workspace_member_id: None,
            api_key_id: None,
            agent_id: None,
            application_id: None,
            main_role_ids: Vec::new(),
            limiting_role_ids: Vec::new(),
            is_system: true,
        }
    }

    pub fn from_auth_context(auth: &AuthContext) -> Self {
        let principal_kind = if auth.token_type == TokenType::ApiKey {
            PermissionPrincipalKind::ApiKey
        } else if auth.agent_id.is_some() {
            PermissionPrincipalKind::Agent
        } else {
            PermissionPrincipalKind::WorkspaceMember
        };
        Self {
            workspace_id: auth.workspace_id,
            principal_kind,
            user_id: auth.user_id,
            user_workspace_id: auth.user_workspace_id,
            workspace_member_id: auth.workspace_member_id,
            api_key_id: auth.api_key_id,
            agent_id: auth.agent_id,
            application_id: None,
            main_role_ids: Vec::new(),
            limiting_role_ids: Vec::new(),
            is_system: false,
        }
    }

    pub async fn load(pool: &PgPool, auth: &AuthContext) -> Result<Self, PermissionError> {
        let mut context = Self::from_auth_context(auth);
        let Some(workspace_id) = context.workspace_id else {
            return Err(PermissionError::new(
                PermissionErrorCode::NoAuthenticationContext,
                "No valid authentication context found",
            ));
        };

        if context.principal_kind == PermissionPrincipalKind::WorkspaceMember
            && context.workspace_member_id.is_none()
        {
            let Some(user_id) = context.user_id else {
                return Err(PermissionError::new(
                    PermissionErrorCode::NoAuthenticationContext,
                    "No valid authentication context found",
                ));
            };
            let member: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM core.\"userWorkspace\" \
                 WHERE \"userId\" = $1 AND \"workspaceId\" = $2",
            )
            .bind(user_id)
            .bind(workspace_id)
            .fetch_optional(pool)
            .await
            .map_err(database_error)?;
            context.user_workspace_id = member.map(|(id,)| id);
            if context.user_workspace_id.is_none() {
                return Err(PermissionError::new(
                    PermissionErrorCode::UserWorkspaceNotFound,
                    "User workspace not found",
                ));
            }
        }

        let (query, principal_id) = match context.principal_kind {
            PermissionPrincipalKind::WorkspaceMember => (
                "SELECT \"roleId\" FROM core.\"roleTarget\" \
                 WHERE \"workspaceId\" = $1 AND \"userWorkspaceId\" = $2 \
                 AND \"isSystemSideEffect\" = false",
                context.user_workspace_id,
            ),
            PermissionPrincipalKind::ApiKey => (
                "SELECT \"roleId\" FROM core.\"roleTarget\" \
                 WHERE \"workspaceId\" = $1 AND \"apiKeyId\" = $2 \
                 AND \"isSystemSideEffect\" = false",
                context.api_key_id,
            ),
            PermissionPrincipalKind::Agent => (
                "SELECT \"roleId\" FROM core.\"roleTarget\" \
                 WHERE \"workspaceId\" = $1 AND \"agentId\" = $2 \
                 AND \"isSystemSideEffect\" = false",
                context.agent_id,
            ),
            PermissionPrincipalKind::Application | PermissionPrincipalKind::System => {
                return Ok(context);
            }
        };
        let Some(principal_id) = principal_id else {
            return Err(PermissionError::new(
                PermissionErrorCode::NoAuthenticationContext,
                "No valid authentication context found",
            ));
        };
        let rows: Vec<(Uuid,)> = sqlx::query_as(query)
            .bind(workspace_id)
            .bind(principal_id)
            .fetch_all(pool)
            .await
            .map_err(database_error)?;
        let role_ids = rows.into_iter().map(|(id,)| id).collect::<Vec<_>>();
        match context.principal_kind {
            PermissionPrincipalKind::Agent => context.limiting_role_ids = role_ids,
            _ => context.main_role_ids = role_ids,
        }
        Ok(context)
    }

    pub fn role_ids(&self) -> BTreeSet<Uuid> {
        self.main_role_ids
            .iter()
            .chain(self.limiting_role_ids.iter())
            .copied()
            .collect()
    }
}

fn database_error(error: sqlx::Error) -> PermissionError {
    PermissionError::new(
        PermissionErrorCode::RecordPermissionQueryFailed,
        error.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::middleware::AuthContext;

    fn auth(token_type: TokenType) -> AuthContext {
        AuthContext {
            token_type,
            user_id: Some(Uuid::new_v4()),
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: Some(Uuid::new_v4()),
            workspace_member_id: Some(Uuid::new_v4()),
            api_key_id: Some(Uuid::new_v4()),
            agent_id: None,
            auth_provider: None,
            is_impersonating: false,
        }
    }

    #[test]
    fn auth_contexts_resolve_the_expected_principal_kind() {
        let member = PermissionsContext::from_auth_context(&auth(TokenType::Access));
        assert_eq!(
            member.principal_kind,
            PermissionPrincipalKind::WorkspaceMember
        );
        let api_key = PermissionsContext::from_auth_context(&auth(TokenType::ApiKey));
        assert_eq!(api_key.principal_kind, PermissionPrincipalKind::ApiKey);
        let mut agent = auth(TokenType::Access);
        agent.agent_id = Some(Uuid::new_v4());
        let agent = PermissionsContext::from_auth_context(&agent);
        assert_eq!(agent.principal_kind, PermissionPrincipalKind::Agent);
    }

    #[test]
    fn system_context_short_circuits_role_resolution() {
        let context = PermissionsContext::system(Some(Uuid::new_v4()));
        assert!(context.is_system);
        assert!(context.role_ids().is_empty());
    }
}
