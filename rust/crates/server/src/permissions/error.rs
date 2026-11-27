#![allow(dead_code)]

use crate::metadata::api::errors::GraphQlError;

macro_rules! permission_error_codes {
    ($($variant:ident => $value:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum PermissionErrorCode {
            $($variant),+
        }

        impl PermissionErrorCode {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }

            pub fn rest_status(self) -> u16 {
                match self {
                    $(Self::$variant => permission_rest_status($value)),+
                }
            }

            pub fn graphql_kind(self) -> GraphqlErrorKind {
                match self {
                    $(Self::$variant => permission_graphql_kind($value)),+
                }
            }
        }
    };
}

permission_error_codes! {
    PermissionDenied => "PERMISSION_DENIED",
    AdminRoleNotFound => "ADMIN_ROLE_NOT_FOUND",
    UserWorkspaceNotFound => "USER_WORKSPACE_NOT_FOUND",
    WorkspaceIdRoleUserWorkspaceMismatch => "WORKSPACE_ID_ROLE_USER_WORKSPACE_MISMATCH",
    TooManyAdminCandidates => "TOO_MANY_ADMIN_CANDIDATES",
    UserWorkspaceAlreadyHasRole => "USER_WORKSPACE_ALREADY_HAS_ROLE",
    WorkspaceMemberNotFound => "WORKSPACE_MEMBER_NOT_FOUND",
    RoleNotFound => "ROLE_NOT_FOUND",
    CannotUnassignLastAdmin => "CANNOT_UNASSIGN_LAST_ADMIN",
    CannotDeleteLastAdminUser => "CANNOT_DELETE_LAST_ADMIN_USER",
    CannotDeleteLastServerAdmin => "CANNOT_DELETE_LAST_SERVER_ADMIN",
    UnknownOperationName => "UNKNOWN_OPERATION_NAME_PERMISSIONS",
    UnknownRequiredPermission => "UNKNOWN_REQUIRED_PERMISSION",
    CannotUpdateSelfRole => "CANNOT_UPDATE_SELF_ROLE",
    CannotDeleteOwnRole => "CANNOT_DELETE_OWN_ROLE",
    CannotRevokeOwnSettingsAccess => "CANNOT_REVOKE_OWN_SETTINGS_ACCESS",
    NoRoleFoundForUserWorkspace => "NO_ROLE_FOUND_FOR_USER_WORKSPACE",
    ApiKeyRoleNotFound => "API_KEY_ROLE_NOT_FOUND",
    NoAuthenticationContext => "NO_AUTHENTICATION_CONTEXT",
    InvalidArg => "INVALID_ARG_PERMISSIONS",
    RoleLabelAlreadyExists => "ROLE_LABEL_ALREADY_EXISTS",
    DefaultRoleNotFound => "DEFAULT_ROLE_NOT_FOUND",
    ObjectMetadataNotFound => "OBJECT_METADATA_NOT_FOUND_PERMISSIONS",
    InvalidSetting => "INVALID_SETTING_PERMISSIONS",
    RoleNotEditable => "ROLE_NOT_EDITABLE",
    RecordNotEditable => "RECORD_NOT_EDITABLE",
    DefaultRoleCannotBeDeleted => "DEFAULT_ROLE_CANNOT_BE_DELETED",
    MemberRoleCannotBeDeleted => "MEMBER_ROLE_CANNOT_BE_DELETED",
    CustomRolesNotEntitled => "CUSTOM_ROLES_NOT_ENTITLED",
    NoPermissionsFoundInDatasource => "NO_PERMISSIONS_FOUND_IN_DATASOURCE",
    CannotAddObjectPermissionOnSystemObject => "CANNOT_ADD_OBJECT_PERMISSION_ON_SYSTEM_OBJECT",
    CannotAddFieldPermissionOnSystemObject => "CANNOT_ADD_FIELD_PERMISSION_ON_SYSTEM_OBJECT",
    MethodNotAllowed => "METHOD_NOT_ALLOWED",
    RawSqlNotAllowed => "RAW_SQL_NOT_ALLOWED",
    CannotGiveWritingPermissionOnNonReadableObject => "CANNOT_GIVE_WRITING_PERMISSION_ON_NON_READABLE_OBJECT",
    CannotGiveWritingPermissionWithoutReadingPermission => "CANNOT_GIVE_WRITING_PERMISSION_WITHOUT_READING_PERMISSION",
    FieldMetadataNotFound => "FIELD_METADATA_NOT_FOUND",
    OnlyFieldRestrictionAllowed => "ONLY_FIELD_RESTRICTION_ALLOWED",
    FieldRestrictionOnlyAllowedOnReadableObject => "FIELD_RESTRICTION_ONLY_ALLOWED_ON_READABLE_OBJECT",
    FieldRestrictionOnUpdateOnlyAllowedOnUpdatableObject => "FIELD_RESTRICTION_ON_UPDATE_ONLY_ALLOWED_ON_UPDATABLE_OBJECT",
    UpsertFieldPermissionFailed => "UPSERT_FIELD_PERMISSION_FAILED",
    PermissionNotFound => "PERMISSION_NOT_FOUND",
    ObjectPermissionNotFound => "OBJECT_PERMISSION_NOT_FOUND",
    FieldPermissionNotFound => "FIELD_PERMISSION_NOT_FOUND",
    EmptyFieldPermissionNotAllowed => "EMPTY_FIELD_PERMISSION_NOT_ALLOWED",
    JoinColumnNameRequired => "JOIN_COLUMN_NAME_REQUIRED",
    CompositeTypeNotFound => "COMPOSITE_TYPE_NOT_FOUND",
    RoleMustHaveAtLeastOneTarget => "ROLE_MUST_HAVE_AT_LEAST_ONE_TARGET",
    RoleCannotBeAssignedToUsers => "ROLE_CANNOT_BE_ASSIGNED_TO_USERS",
    ApplicationRoleNotFound => "APPLICATION_ROLE_NOT_FOUND",
    RoleBelongsToAnotherApplication => "ROLE_BELONGS_TO_ANOTHER_APPLICATION",
    MarketplaceApplicationRoleAssignability => "MARKETPLACE_APPLICATION_ROLE_ASSIGNABILITY",
    MarketplaceApplicationRoleRestriction => "MARKETPLACE_APPLICATION_ROLE_RESTRICTION",
    RecordPermissionQueryInvalid => "RECORD_PERMISSION_QUERY_INVALID",
    RlsValidationFailed => "RLS_VALIDATION_FAILED",
    RlsPredicateFieldNotFound => "RLS_PREDICATE_FIELD_NOT_FOUND",
    RecordPermissionQueryFailed => "RECORD_PERMISSION_QUERY_FAILED",
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphqlErrorKind {
    Forbidden,
    UserInput,
    NotFound,
    Internal,
}

#[derive(Debug, Clone)]
pub struct PermissionError {
    pub code: PermissionErrorCode,
    pub message: String,
}

impl PermissionError {
    pub fn new(code: PermissionErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn denied(message: impl Into<String>) -> Self {
        Self::new(PermissionErrorCode::PermissionDenied, message)
    }

    pub fn record_not_editable() -> Self {
        Self::new(
            PermissionErrorCode::RecordNotEditable,
            "Record is not editable",
        )
    }
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for PermissionError {}

impl From<PermissionError> for GraphQlError {
    fn from(error: PermissionError) -> Self {
        let graphql_error = match error.code.graphql_kind() {
            GraphqlErrorKind::Forbidden => GraphQlError::forbidden(error.message),
            GraphqlErrorKind::UserInput => GraphQlError::user_input(error.message),
            GraphqlErrorKind::NotFound => GraphQlError::not_found(error.message),
            GraphqlErrorKind::Internal => GraphQlError::internal(error.message),
        };
        if matches!(
            error.code,
            PermissionErrorCode::PermissionDenied | PermissionErrorCode::NoAuthenticationContext
        ) {
            graphql_error.with_sub_code(error.code.as_str())
        } else {
            graphql_error
        }
    }
}

const FORBIDDEN_CODES: &[&str] = &[
    "PERMISSION_DENIED",
    "NO_AUTHENTICATION_CONTEXT",
    "ROLE_LABEL_ALREADY_EXISTS",
    "CANNOT_UNASSIGN_LAST_ADMIN",
    "CANNOT_UPDATE_SELF_ROLE",
    "CANNOT_DELETE_OWN_ROLE",
    "CANNOT_REVOKE_OWN_SETTINGS_ACCESS",
    "CANNOT_DELETE_LAST_ADMIN_USER",
    "CANNOT_DELETE_LAST_SERVER_ADMIN",
    "ROLE_NOT_EDITABLE",
    "RECORD_NOT_EDITABLE",
    "DEFAULT_ROLE_CANNOT_BE_DELETED",
    "MEMBER_ROLE_CANNOT_BE_DELETED",
    "CUSTOM_ROLES_NOT_ENTITLED",
    "CANNOT_ADD_OBJECT_PERMISSION_ON_SYSTEM_OBJECT",
    "CANNOT_ADD_FIELD_PERMISSION_ON_SYSTEM_OBJECT",
];

const BAD_REQUEST_CODES: &[&str] = &[
    "INVALID_ARG_PERMISSIONS",
    "INVALID_SETTING_PERMISSIONS",
    "CANNOT_GIVE_WRITING_PERMISSION_ON_NON_READABLE_OBJECT",
    "CANNOT_GIVE_WRITING_PERMISSION_WITHOUT_READING_PERMISSION",
    "ONLY_FIELD_RESTRICTION_ALLOWED",
    "FIELD_RESTRICTION_ONLY_ALLOWED_ON_READABLE_OBJECT",
    "FIELD_RESTRICTION_ON_UPDATE_ONLY_ALLOWED_ON_UPDATABLE_OBJECT",
    "EMPTY_FIELD_PERMISSION_NOT_ALLOWED",
    "ROLE_MUST_HAVE_AT_LEAST_ONE_TARGET",
    "ROLE_CANNOT_BE_ASSIGNED_TO_USERS",
    "MARKETPLACE_APPLICATION_ROLE_ASSIGNABILITY",
    "MARKETPLACE_APPLICATION_ROLE_RESTRICTION",
    "RECORD_PERMISSION_QUERY_INVALID",
    "RLS_VALIDATION_FAILED",
];

const NOT_FOUND_CODES: &[&str] = &[
    "ROLE_NOT_FOUND",
    "OBJECT_METADATA_NOT_FOUND_PERMISSIONS",
    "FIELD_METADATA_NOT_FOUND",
    "FIELD_PERMISSION_NOT_FOUND",
    "PERMISSION_NOT_FOUND",
];

fn permission_rest_status(code: &str) -> u16 {
    if FORBIDDEN_CODES.contains(&code) {
        403
    } else if BAD_REQUEST_CODES.contains(&code) {
        400
    } else if NOT_FOUND_CODES.contains(&code) {
        404
    } else {
        500
    }
}

fn permission_graphql_kind(code: &str) -> GraphqlErrorKind {
    if FORBIDDEN_CODES.contains(&code) {
        GraphqlErrorKind::Forbidden
    } else if BAD_REQUEST_CODES.contains(&code) {
        GraphqlErrorKind::UserInput
    } else if NOT_FOUND_CODES.contains(&code) {
        GraphqlErrorKind::NotFound
    } else {
        GraphqlErrorKind::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_status_table_matches_upstream_groups() {
        assert_eq!(PermissionErrorCode::PermissionDenied.rest_status(), 403);
        assert_eq!(PermissionErrorCode::RecordNotEditable.rest_status(), 403);
        assert_eq!(PermissionErrorCode::InvalidArg.rest_status(), 400);
        assert_eq!(PermissionErrorCode::RlsValidationFailed.rest_status(), 400);
        assert_eq!(PermissionErrorCode::RoleNotFound.rest_status(), 404);
        assert_eq!(
            PermissionErrorCode::ObjectMetadataNotFound.rest_status(),
            404
        );
        assert_eq!(PermissionErrorCode::UnknownOperationName.rest_status(), 500);
        assert_eq!(
            PermissionErrorCode::RecordPermissionQueryFailed.rest_status(),
            500
        );
    }

    #[test]
    fn graphql_kinds_match_upstream_handler_groups() {
        assert_eq!(
            PermissionErrorCode::PermissionDenied.graphql_kind(),
            GraphqlErrorKind::Forbidden
        );
        assert_eq!(
            PermissionErrorCode::OnlyFieldRestrictionAllowed.graphql_kind(),
            GraphqlErrorKind::UserInput
        );
        assert_eq!(
            PermissionErrorCode::FieldMetadataNotFound.graphql_kind(),
            GraphqlErrorKind::NotFound
        );
        assert_eq!(
            PermissionErrorCode::UnknownOperationName.graphql_kind(),
            GraphqlErrorKind::Internal
        );
    }
}
