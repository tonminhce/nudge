use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::extract::Query;
use axum::extract::Request;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::middleware::AuthContext;
use crate::file::sign_search_avatars;
use crate::file::FileUrlSigner;
use crate::metadata::api::errors::GraphQlError;
use crate::metadata::api::graphql::parse_request;
use crate::metadata::api::graphql::select_operation;
use crate::metadata::api::graphql::ExecutionRequest;
use crate::metadata::api::graphql::FieldSelection;
use crate::metadata::api::graphql::GraphqlRequestBody;
use crate::metadata::api::graphql::OperationKind;
use crate::metadata::api::service::load_flat_maps;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::names::camel_case;
use crate::metadata::names::compute_composite_column_name;
use crate::metadata::names::pascal_case;
use crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER;
use crate::permissions::current_user_workspace_permissions_for_auth;
use crate::permissions::records_permissions_for_auth;
use crate::query::cursor::encode_cursor_data;
use crate::query::runner::create_many;
use crate::query::runner::create_one_with_snapshot;
use crate::query::runner::find_duplicates;
use crate::query::runner::find_many;
use crate::query::runner::find_one;
use crate::query::runner::find_one::FindOneArgs;
use crate::query::runner::merge_many;
use crate::query::runner::run_mutation_one;
use crate::query::runner::run_mutation_one_with_snapshot;
use crate::query::runner::search;
use crate::query::runner::ActorContext;
use crate::query::runner::CreateArgs;
use crate::query::runner::FilteredMutationKind;
use crate::query::runner::FindManyArgs;
use crate::query::runner::MergeManyArgs;
use crate::query::runner::NestedRelationSelection;
use crate::query::runner::RunnerSchema;
use crate::query::runner::SearchArgs;
use crate::state::AppState;

use super::schema::classify_root;
use super::schema::ResolverMethod;
use super::schema::RootClassification;
use super::schema::WorkspaceSchema;

pub const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(graphql_endpoint).get(graphql_get_endpoint))
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct GraphqlQueryParameters {
    #[serde(default)]
    query: Option<String>,
    #[serde(rename = "operationName", default)]
    operation_name: Option<String>,
    #[serde(default)]
    variables: Option<String>,
}

async fn graphql_get_endpoint(
    State(state): State<AppState>,
    Query(parameters): Query<GraphqlQueryParameters>,
    request: Request,
) -> Response {
    let variables = parameters
        .variables
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(Value::Null);

    execute_graphql(
        state,
        request,
        GraphqlRequestBody {
            query: parameters.query,
            operation_name: parameters.operation_name,
            variables: Some(variables),
            extensions: None,
        },
        RequestBodyKind::Json,
    )
    .await
}

async fn graphql_endpoint(State(state): State<AppState>, request: Request) -> Response {
    execute_graphql(
        state,
        request,
        GraphqlRequestBody {
            query: None,
            operation_name: None,
            variables: None,
            extensions: None,
        },
        RequestBodyKind::Http,
    )
    .await
}

enum RequestBodyKind {
    Http,
    Json,
}

async fn execute_graphql(
    state: AppState,
    request: Request,
    mut query_parameters: GraphqlRequestBody,
    body_kind: RequestBodyKind,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let auth = match AuthContext::from_request_parts(&mut parts, &state).await {
        Ok(context) => context,
        Err(error) => return error_response(&parts.headers, &[auth_error(&error)]),
    };

    if auth.agent_id.is_some() {
        return error_response(
            &parts.headers,
            &[
                GraphQlError::forbidden("This token is only accepted by MCP")
                    .with_sub_code("INVALID_AUTH_CONTEXT"),
            ],
        );
    }

    let Some(workspace_id) = auth.workspace_id else {
        return error_response(
            &parts.headers,
            &[GraphQlError::forbidden("Could not retrieve workspace ID")
                .with_sub_code("WORKSPACE_NOT_DEFINED")],
        );
    };

    let body_kind_name = match body_kind {
        RequestBodyKind::Http => "http",
        RequestBodyKind::Json => "json",
    };
    let _ = body_kind_name;
    let content_type = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();

    if let RequestBodyKind::Http = body_kind {
        let bytes = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return error_response(
                    &parts.headers,
                    &[GraphQlError::validation(
                        "Request body was too large or could not be read.",
                    )],
                )
            }
        };

        if content_type == "application/graphql" {
            query_parameters.query = Some(String::from_utf8_lossy(&bytes).to_string());
        } else {
            let parsed: Result<GraphqlRequestBody, _> = serde_json::from_slice(&bytes);

            match parsed {
                Ok(parsed) => query_parameters = parsed,
                Err(_) => {
                    return error_response(
                        &parts.headers,
                        &[GraphQlError::validation(
                            "POST body missing, invalid, or not JSON.",
                        )],
                    )
                }
            }
        }
    }

    let execution_request = match parse_request(query_parameters) {
        Ok(request) => request,
        Err(error) => return error_response(&parts.headers, &[error]),
    };

    let maps = match load_flat_maps(&state, workspace_id).await {
        Ok(maps) => maps,
        Err(error) => return error_response(&parts.headers, &[error]),
    };
    let schema = WorkspaceSchema::build(&maps);

    if let Some(document) = execution_request.document.as_ref() {
        let fields = document
            .operations
            .iter()
            .flat_map(|operation| operation.fields.iter())
            .collect::<Vec<_>>();
        let has_core = fields
            .iter()
            .any(|field| classify_root(&field.name, &schema) == RootClassification::Core);
        let has_workspace = fields
            .iter()
            .any(|field| classify_root(&field.name, &schema) == RootClassification::Workspace);

        if has_core && has_workspace {
            return error_response(
                &parts.headers,
                &[GraphQlError::user_input(
                    "This query cannot be executed as a single request. Please split it into separate queries.",
                )
                .with_sub_code("CORE_WORKSPACE_MIX")],
            );
        }
    }

    let selected = match select_operation(&execution_request) {
        Ok(selected) => selected,
        Err(error) => return error_response(&parts.headers, &[error]),
    };

    let classification = classify_root(&selected.field.name, &schema);

    match classification {
        RootClassification::Introspection => {
            let value = introspection_value(&selected.field, &schema);
            data_response(&parts.headers, selected.field.response_key(), value)
        }
        RootClassification::Workspace => {
            match execute_workspace(
                &state,
                workspace_id,
                &schema,
                &maps,
                &auth,
                &selected.field,
                &execution_request,
                &parts.headers,
                selected.kind == OperationKind::Query,
            )
            .await
            {
                Ok(value) => data_response(&parts.headers, selected.field.response_key(), value),
                Err(error) => error_response_for_key(
                    &parts.headers,
                    Some(selected.field.response_key()),
                    &[error],
                ),
            }
        }
        RootClassification::Core => {
            if has_workspace_field(&execution_request, &schema) {
                return error_response(
                    &parts.headers,
                    &[GraphQlError::user_input(
                        "This query cannot be executed as a single request. Please split it into separate queries.",
                    )
                    .with_sub_code("CORE_WORKSPACE_MIX")],
                );
            }

            match core_operation(&state, workspace_id, &auth, &selected.field).await {
                Ok(value) => data_response(&parts.headers, selected.field.response_key(), value),
                Err(error) => error_response_for_key(
                    &parts.headers,
                    Some(selected.field.response_key()),
                    &[error],
                ),
            }
        }
    }
}

fn has_workspace_field(request: &ExecutionRequest, schema: &WorkspaceSchema) -> bool {
    request
        .document
        .as_ref()
        .map(|document| {
            document
                .operations
                .iter()
                .flat_map(|operation| operation.fields.iter())
                .any(|field| classify_root(&field.name, schema) == RootClassification::Workspace)
        })
        .unwrap_or(false)
}

fn auth_error(error: &AuthError) -> GraphQlError {
    if error.code == "UNAUTHENTICATED" {
        return GraphQlError::unauthenticated("Authentication required").with_extensions(json!({
            "subCode": "UNAUTHENTICATED",
            "userFriendlyMessage": {
                "id": "z+7x/s",
                "message": "You must be authenticated to perform this action."
            }
        }));
    }
    let message = error
        .messages
        .first()
        .cloned()
        .unwrap_or_else(|| "Authentication required".to_string());
    let mut mapped = match error.code {
        "UNAUTHENTICATED" | "USER_NOT_FOUND" | "WORKSPACE_NOT_FOUND" => {
            GraphQlError::unauthenticated(message)
        }
        "FORBIDDEN_EXCEPTION" | "INVALID_JWT_TOKEN_TYPE" | "USER_ALREADY_EXISTS" => {
            GraphQlError::forbidden(message)
        }
        "INVALID_INPUT" => GraphQlError::user_input(message),
        _ => GraphQlError::internal(message),
    };

    mapped = mapped.with_sub_code(error.code);
    mapped
}

fn error_response(_headers: &HeaderMap, errors: &[GraphQlError]) -> Response {
    error_response_for_key(_headers, None, errors)
}

fn error_response_for_key(
    _headers: &HeaderMap,
    root_key: Option<&str>,
    errors: &[GraphQlError],
) -> Response {
    let errors = errors.iter().map(GraphQlError::to_json).collect::<Vec<_>>();
    let payload = match root_key {
        Some(key) => json!({
            "data": { key: Value::Null },
            "errors": errors,
        }),
        None => json!({ "errors": errors }),
    };
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        Json(payload),
    )
        .into_response()
}

fn runner_graphql_error(error: crate::query::runner::RunnerError) -> GraphQlError {
    let message = error.message().to_string();
    let code = error.code();

    if code == "RECORD_NOT_FOUND" {
        return record_not_found_graphql_error();
    }

    if code == "RECORD_DOES_NOT_SATISFY_RLS" {
        let mut error = GraphQlError::user_input(message);
        error.extensions = json!({
            "userFriendlyMessage": "Record does not satisfy security constraints."
        });
        return error;
    }

    if matches!(code, "PERMISSION_DENIED" | "RECORD_NOT_EDITABLE") {
        return friendly_graphql_error(
            GraphQlError::forbidden(message),
            code,
            "User does not have permission.",
        );
    }

    if matches!(
        code,
        "ARGS_CONFLICT"
            | "INVALID_ARGS_FIRST"
            | "INVALID_ARGS_LAST"
            | "INVALID_QUERY_INPUT"
            | "INVALID_ARGS_DATA"
            | "INVALID_ARGS_FILTER"
            | "UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT"
            | "INVALID_CURSOR"
            | "TOO_MANY_RECORDS_TO_UPDATE"
            | "BAD_REQUEST"
            | "TOO_COMPLEX_QUERY"
            | "MISSING_TIMEZONE_FOR_DATE_GROUP_BY"
            | "INVALID_TIMEZONE"
            | "UNSUPPORTED_OPERATOR"
            | "UNKNOWN_COLUMN"
            | "UNKNOWN_RELATION"
            | "UNKNOWN_SUB_FIELD"
            | "INVALID_DIRECTION"
            | "LABEL_IDENTIFIER_FIELD_NOT_FOUND"
            | "OBJECT_METADATA_NOT_FOUND"
    ) {
        return friendly_graphql_error(
            GraphQlError::user_input(message),
            code,
            "An error occurred.",
        );
    }

    if code == "UNKNOWN_OBJECT" {
        return friendly_graphql_error(
            GraphQlError::user_input(message),
            code,
            "An error occurred.",
        );
    }

    GraphQlError::internal(message).with_sub_code(code)
}

fn friendly_graphql_error(
    mut error: GraphQlError,
    sub_code: &str,
    user_friendly_message: &str,
) -> GraphQlError {
    if let Some(object) = error.extensions.as_object_mut() {
        object.insert(
            String::from("userFriendlyMessage"),
            Value::String(user_friendly_message.to_string()),
        );
    }

    error.with_sub_code(sub_code)
}

fn record_not_found_graphql_error() -> GraphQlError {
    GraphQlError::not_found("Record not found").with_extensions(json!({
        "subCode": "RECORD_NOT_FOUND",
        "userFriendlyMessage": "This record does not exist or has been deleted.",
    }))
}

fn data_response(_headers: &HeaderMap, key: &str, value: Value) -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        Json(json!({ "data": { key: value } })),
    )
        .into_response()
}

const SUPPORTED_LOCALES: &[&str] = &[
    "en",
    "pseudo-en",
    "af-ZA",
    "ar-SA",
    "ca-ES",
    "cs-CZ",
    "da-DK",
    "de-DE",
    "el-GR",
    "es-ES",
    "fa-IR",
    "fi-FI",
    "fr-FR",
    "he-IL",
    "hu-HU",
    "hy-AM",
    "it-IT",
    "ja-JP",
    "ko-KR",
    "nl-NL",
    "no-NO",
    "pl-PL",
    "pt-BR",
    "pt-PT",
    "ro-RO",
    "ru-RU",
    "sr-Cyrl",
    "sr-Latn",
    "sv-SE",
    "tr-TR",
    "uk-UA",
    "uz-UZ",
    "vi-VN",
    "zh-CN",
    "zh-TW",
];

fn normalize_locale(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::from("en");
    };
    if let Some(locale) = SUPPORTED_LOCALES
        .iter()
        .find(|locale| locale.eq_ignore_ascii_case(value))
    {
        return (*locale).to_string();
    }
    let language = value
        .trim()
        .split('-')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    SUPPORTED_LOCALES
        .iter()
        .find(|locale| {
            locale
                .split('-')
                .next()
                .unwrap_or_default()
                .eq_ignore_ascii_case(&language)
        })
        .unwrap_or(&"en")
        .to_string()
}

async fn search_locale(
    pool: &sqlx::PgPool,
    auth: &AuthContext,
    headers: &HeaderMap,
) -> Result<String, GraphQlError> {
    let header_locale = headers
        .get("x-locale")
        .and_then(|value| value.to_str().ok());
    if let Some(user_workspace_id) = auth.user_workspace_id {
        let locale: Option<Option<String>> =
            sqlx::query_scalar("SELECT locale FROM core.\"userWorkspace\" WHERE id = $1")
                .bind(user_workspace_id)
                .fetch_optional(pool)
                .await
                .map_err(|error| GraphQlError::internal(error.to_string()))?;
        if let Some(locale) = locale.flatten() {
            return Ok(normalize_locale(Some(&locale)));
        }
    }
    Ok(normalize_locale(header_locale))
}

async fn search_application_catalogs(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    locale: &str,
) -> Result<BTreeMap<Uuid, Value>, GraphQlError> {
    let rows = sqlx::query_as::<_, (Uuid, Value)>(
        "SELECT a.id, t.messages FROM core.application a \
         JOIN core.\"applicationTranslation\" t \
         ON t.\"applicationRegistrationId\" = a.\"applicationRegistrationId\" \
         WHERE a.\"workspaceId\" = $1 AND t.locale = $2 \
         AND a.\"deletedAt\" IS NULL AND t.\"deletedAt\" IS NULL \
         AND a.\"universalIdentifier\"::text <> $3",
    )
    .bind(workspace_id)
    .bind(locale)
    .bind(TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER)
    .fetch_all(pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?;
    Ok(rows.into_iter().collect())
}

#[allow(clippy::too_many_arguments)]
async fn execute_workspace(
    state: &AppState,
    workspace_id: Uuid,
    schema: &WorkspaceSchema,
    maps: &crate::metadata::flat::FlatWorkspaceMaps,
    auth: &AuthContext,
    field: &FieldSelection,
    request: &ExecutionRequest,
    headers: &HeaderMap,
    is_query: bool,
) -> Result<Value, GraphQlError> {
    let _ = request;
    let permissions = records_permissions_for_auth(state, auth)
        .await
        .map_err(GraphQlError::from)?;
    let runner_schema = Arc::new(match permissions {
        Some(permissions) => RunnerSchema::build_with_permissions(
            &workspace_id,
            schema.objects(),
            &maps.field_metadata_maps,
            &maps.index_metadata_maps,
            permissions,
            auth.workspace_member_id,
        )
        .map_err(runner_graphql_error)?
        .with_user_workspace_id(auth.user_workspace_id),
        None => RunnerSchema::build(
            &workspace_id,
            schema.objects(),
            &maps.field_metadata_maps,
            &maps.index_metadata_maps,
        )
        .map_err(runner_graphql_error)?
        .with_user_workspace_id(auth.user_workspace_id),
    });
    let actor = Some(ActorContext::from_auth(auth));

    if field.name == "search" {
        if !is_query {
            return Err(GraphQlError::user_input(
                "Operation kind does not match the workspace resolver.",
            ));
        }
        let locale = search_locale(&state.core_pool, auth, headers).await?;
        let application_catalogs =
            search_application_catalogs(&state.core_pool, workspace_id, &locale).await?;
        let mut result = search(
            &state.core_pool,
            runner_schema.clone(),
            schema.objects(),
            &parse_search_args(field, locale, application_catalogs)?,
            state.config.search_ilike_fallback_timeout_ms,
            state.config.allow_requests_to_twenty_icons,
        )
        .await
        .map_err(runner_graphql_error)?;
        let signer = FileUrlSigner::new(
            state.jwt.clone(),
            state.config.server_url.clone(),
            state.config.file_token_expires_in_secs,
        );
        sign_search_avatars(&mut result, &signer, workspace_id)
            .await
            .map_err(|error| auth_error(&error))?;
        return Ok(project_search_connection(&result, &field.selection));
    }
    let resolver = schema.resolver(&field.name).ok_or_else(|| {
        GraphQlError::user_input(format!("Unknown workspace field {}", field.name))
    })?;

    if is_query != resolver.method.is_query() {
        return Err(GraphQlError::user_input(
            "Operation kind does not match the workspace resolver.",
        ));
    }

    match resolver.method {
        ResolverMethod::FindMany => {
            find_many_root(state, schema, &runner_schema, field, resolver).await
        }
        ResolverMethod::FindOne => {
            find_one_root(state, schema, &runner_schema, field, resolver).await
        }
        ResolverMethod::FindDuplicates => {
            find_duplicates_root(state, schema, &runner_schema, field, resolver).await
        }
        ResolverMethod::CreateOne => {
            let data = required_object(field, "data")?;
            let upsert = optional_bool_arg(field, "upsert");
            let after = create_one_with_snapshot(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                data,
                upsert,
                true,
                actor.as_ref(),
            )
            .await
            .map_err(runner_graphql_error)?;
            let id = after
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or_else(record_not_found_graphql_error)?;
            let workspace_id = runner_schema
                .object(&resolver.object_name_singular)
                .map_err(runner_graphql_error)?
                .object
                .workspace_id;
            state.event_streams.publish_record(
                workspace_id,
                "created",
                &resolver.object_name_singular,
                &id.to_string(),
                json!({"after": after}),
                runner_schema.as_ref(),
                actor.as_ref(),
            );

            find_record(state, schema, &runner_schema, field, resolver, id, false).await
        }
        ResolverMethod::CreateMany => {
            let data = field
                .argument("data")
                .and_then(Value::as_array)
                .cloned()
                .ok_or_else(|| GraphQlError::user_input("CreateMany requires data"))?;
            let records = data
                .into_iter()
                .filter_map(|value| value.as_object().cloned())
                .collect::<Vec<_>>();
            let ids = create_many(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                &CreateArgs {
                    records,
                    upsert: optional_bool_arg(field, "upsert"),
                    backfill_position_undefined: true,
                    actor: actor.clone(),
                },
            )
            .await
            .map_err(runner_graphql_error)?;
            let filters = ids.iter().map(|id| id_filter(*id)).collect::<Vec<_>>();

            find_many(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                &FindManyArgs {
                    filter: Some(Value::Object(single_entry("or", Value::Array(filters)))),
                    selected_field_names: root_record_fields(schema, field, resolver),
                    ..Default::default()
                },
                false,
                &nested_relations(schema, field, resolver),
            )
            .await
            .map(|result| Value::Array(result.records))
            .map_err(runner_graphql_error)
        }
        ResolverMethod::UpdateOne => {
            mutate_existing_record(
                state,
                schema,
                &runner_schema,
                field,
                resolver,
                FilteredMutationKind::Update,
                field.argument("data").cloned(),
                actor.as_ref(),
                false,
            )
            .await
        }
        ResolverMethod::DeleteOne => {
            let id = required_id(field)?;

            let snapshot = run_mutation_one_with_snapshot(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                FilteredMutationKind::SoftDelete,
                Some(id_filter_value(id)),
                None,
                actor.as_ref(),
                false,
                Vec::new(),
            )
            .await
            .map_err(runner_graphql_error)?;
            let properties = crate::events::record_mutation_properties(
                &snapshot.before,
                snapshot.after.as_ref(),
            )
            .unwrap_or_else(|| json!({"before": snapshot.before, "after": snapshot.after}));
            let workspace_id = runner_schema
                .object(&resolver.object_name_singular)
                .map_err(runner_graphql_error)?
                .object
                .workspace_id;
            state.event_streams.publish_record(
                workspace_id,
                "deleted",
                &resolver.object_name_singular,
                &id.to_string(),
                properties,
                runner_schema.as_ref(),
                actor.as_ref(),
            );

            find_record(state, schema, &runner_schema, field, resolver, id, true).await
        }
        ResolverMethod::DestroyOne => {
            let id = required_id(field)?;
            let before =
                find_record(state, schema, &runner_schema, field, resolver, id, true).await?;

            run_mutation_one(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                FilteredMutationKind::Destroy,
                Some(id_filter_value(id)),
                None,
                actor.as_ref(),
                true,
                vec![String::from("id")],
            )
            .await
            .map_err(runner_graphql_error)?;

            Ok(before)
        }
        ResolverMethod::RestoreOne => {
            mutate_existing_record(
                state,
                schema,
                &runner_schema,
                field,
                resolver,
                FilteredMutationKind::Restore,
                None,
                actor.as_ref(),
                true,
            )
            .await
        }
        ResolverMethod::MergeMany => {
            merge_many_root(
                state,
                schema,
                &runner_schema,
                field,
                resolver,
                actor.as_ref(),
            )
            .await
        }
        ResolverMethod::GroupBy => Err(GraphQlError::user_input(
            "GroupBy is executed by the group-by runner endpoint in a following slice",
        )),
    }
}

async fn find_many_root(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Result<Value, GraphQlError> {
    let mut args = FindManyArgs {
        filter: field.argument("filter").cloned(),
        order_by: field.argument("orderBy").cloned(),
        first: optional_usize(field, "first"),
        last: optional_usize(field, "last"),
        after: optional_string_arg(field, "after"),
        before: optional_string_arg(field, "before"),
        offset: field.argument("offset").and_then(Value::as_i64),
        selected_field_names: root_record_fields(schema, field, resolver),
        ..Default::default()
    };

    if args.first.is_none() && args.last.is_none() {
        args.first = Some(crate::query::runner::DEFAULT_PAGE_SIZE);
    }

    let include_total_count = field
        .selection
        .iter()
        .any(|selection| selection.name == "totalCount");
    let result = find_many(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        &args,
        include_total_count,
        &nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?;
    let object_type = pascal_case(&resolver.object_name_singular);
    let edges = result
        .records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let id = record.get("id").and_then(Value::as_str).unwrap_or_default();
            let cursor = result.cursors.get(id).cloned().unwrap_or_else(|| {
                encode_cursor_data(&Map::from_iter([(
                    String::from("id"),
                    Value::String(id.to_string()),
                )]))
            });

            json!({
                "node": record,
                "cursor": cursor,
                "_index": index,
            })
        })
        .collect::<Vec<_>>();
    let connection = json!({
        "edges": edges,
        "pageInfo": {
            "startCursor": result.page_info.start_cursor,
            "endCursor": result.page_info.end_cursor,
            "hasNextPage": result.page_info.has_next_page,
            "hasPreviousPage": result.page_info.has_previous_page,
        },
        "totalCount": result.total_count,
        "__typename": format!("{object_type}Connection"),
    });

    Ok(project_connection(
        &connection,
        &field.selection,
        &object_type,
    ))
}

fn parse_search_args(
    field: &FieldSelection,
    locale: String,
    application_catalogs: BTreeMap<Uuid, Value>,
) -> Result<SearchArgs, GraphQlError> {
    let search_input = field
        .argument("searchInput")
        .and_then(Value::as_str)
        .ok_or_else(|| GraphQlError::user_input("searchInput is required"))?
        .to_string();
    let limit = field
        .argument("limit")
        .and_then(Value::as_u64)
        .ok_or_else(|| GraphQlError::user_input("limit is required"))?;
    if limit > 100 {
        return Err(GraphQlError::user_input("Limit cannot exceed 100 items"));
    }
    let filter = field.argument("filter").filter(|value| !value.is_null());
    if let Some(filter) = filter {
        validate_search_filter(filter)?;
    }
    Ok(SearchArgs {
        search_input,
        limit: limit as usize,
        after: optional_string_arg(field, "after").filter(|value| !value.is_empty()),
        included_object_name_singulars: optional_string_list(field, "includedObjectNameSingulars")?,
        excluded_object_name_singulars: optional_string_list(field, "excludedObjectNameSingulars")?,
        filter: filter.cloned(),
        locale,
        application_catalogs: Arc::new(application_catalogs),
    })
}

fn optional_string_list(field: &FieldSelection, name: &str) -> Result<Vec<String>, GraphQlError> {
    let Some(value) = field.argument(name) else {
        return Ok(Vec::new());
    };
    if value.is_null() {
        return Ok(Vec::new());
    }
    value
        .as_array()
        .ok_or_else(|| GraphQlError::user_input(format!("{name} must be a list of strings")))?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                GraphQlError::user_input(format!("{name} must be a list of strings"))
            })
        })
        .collect()
}

fn validate_search_filter(value: &Value) -> Result<(), GraphQlError> {
    if value.is_null() {
        return Ok(());
    }
    let fields = value
        .as_object()
        .ok_or_else(|| search_filter_error("ObjectRecordFilterInput must be an object"))?;
    for (name, value) in fields {
        match name.as_str() {
            "and" | "or" => validate_search_filter_list(value)?,
            "not" => validate_search_filter(value)?,
            "id" => validate_uuid_filter(value)?,
            "createdAt" | "updatedAt" | "deletedAt" => validate_date_time_filter(value)?,
            _ => {
                return Err(search_filter_error(format!(
                    "Field \"{name}\" is not defined by type \"ObjectRecordFilterInput\"."
                )));
            }
        }
    }
    Ok(())
}

fn validate_search_filter_list(value: &Value) -> Result<(), GraphQlError> {
    if value.is_null() {
        return Ok(());
    }
    let values = value
        .as_array()
        .ok_or_else(|| search_filter_error("Logical filter must be a list"))?;
    for value in values {
        validate_search_filter(value)?;
    }
    Ok(())
}

fn validate_uuid_filter(value: &Value) -> Result<(), GraphQlError> {
    validate_scalar_filter(value, "UUIDFilter", |value| {
        Uuid::parse_str(value).map(|_| ()).map_err(|_| ())
    })
}

fn validate_date_time_filter(value: &Value) -> Result<(), GraphQlError> {
    validate_scalar_filter(value, "DateTimeFilter", |value| {
        chrono::DateTime::parse_from_rfc3339(value)
            .map(|_| ())
            .map_err(|_| ())
    })
}

fn validate_scalar_filter(
    value: &Value,
    type_name: &str,
    validate_scalar: impl Fn(&str) -> Result<(), ()>,
) -> Result<(), GraphQlError> {
    if value.is_null() {
        return Ok(());
    }
    let fields = value
        .as_object()
        .ok_or_else(|| search_filter_error(format!("{type_name} must be an object")))?;
    for (name, value) in fields {
        if value.is_null() {
            continue;
        }
        match name.as_str() {
            "in" => {
                let values = value.as_array().ok_or_else(|| {
                    search_filter_error(format!("Field \"{name}\" in {type_name} must be a list"))
                })?;
                for value in values {
                    if let Some(value) = value.as_str() {
                        validate_scalar(value).map_err(|_| {
                            search_filter_error(format!(
                                "Invalid value for field \"{name}\" in {type_name}."
                            ))
                        })?;
                    } else if !value.is_null() {
                        return Err(search_filter_error(format!(
                            "Field \"{name}\" in {type_name} must contain scalar values."
                        )));
                    }
                }
            }
            "is" => {
                if !matches!(value.as_str(), Some("NULL" | "NOT_NULL")) {
                    return Err(search_filter_error(format!(
                        "Invalid value for field \"is\" in {type_name}."
                    )));
                }
            }
            "eq" | "gt" | "gte" | "lt" | "lte" | "neq" => {
                let value = value.as_str().ok_or_else(|| {
                    search_filter_error(format!(
                        "Field \"{name}\" in {type_name} must be a scalar."
                    ))
                })?;
                validate_scalar(value).map_err(|_| {
                    search_filter_error(format!(
                        "Invalid value for field \"{name}\" in {type_name}."
                    ))
                })?;
            }
            "gtField" | "gteField" | "ltField" | "lteField" if type_name == "DateTimeFilter" => {
                if !value.is_string() {
                    return Err(search_filter_error(format!(
                        "Field \"{name}\" in {type_name} must be a string."
                    )));
                }
            }
            _ => {
                return Err(search_filter_error(format!(
                    "Field \"{name}\" is not defined by type \"{type_name}\"."
                )));
            }
        }
    }
    Ok(())
}

fn search_filter_error(message: impl Into<String>) -> GraphQlError {
    GraphQlError::user_input(message)
}

fn project_search_connection(value: &Value, selections: &[FieldSelection]) -> Value {
    let mut output = Map::new();
    for selection in selections {
        let key = selection.response_key().to_string();
        match selection.name.as_str() {
            "edges" => {
                let edges = value
                    .get("edges")
                    .and_then(Value::as_array)
                    .map(|edges| {
                        edges
                            .iter()
                            .map(|edge| {
                                let mut projected = Map::new();
                                for edge_selection in &selection.selection {
                                    let edge_key = edge_selection.response_key().to_string();
                                    match edge_selection.name.as_str() {
                                        "node" => {
                                            projected.insert(
                                                edge_key,
                                                project_record(
                                                    edge.get("node").unwrap_or(&Value::Null),
                                                    &edge_selection.selection,
                                                    "SearchRecord",
                                                ),
                                            );
                                        }
                                        "cursor" | "__typename" => {
                                            let value = if edge_selection.name == "__typename" {
                                                Value::String(String::from("SearchResultEdge"))
                                            } else {
                                                edge.get("cursor").cloned().unwrap_or(Value::Null)
                                            };
                                            projected.insert(edge_key, value);
                                        }
                                        _ => {}
                                    }
                                }
                                Value::Object(projected)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                output.insert(key, Value::Array(edges));
            }
            "pageInfo" => {
                output.insert(
                    key,
                    project_plain(
                        value.get("pageInfo"),
                        &selection.selection,
                        "SearchResultPageInfo",
                    ),
                );
            }
            "__typename" => {
                output.insert(key, Value::String(String::from("SearchResultConnection")));
            }
            _ => {}
        }
    }
    Value::Object(output)
}

fn project_connection(
    connection: &Value,
    selections: &[FieldSelection],
    object_type: &str,
) -> Value {
    if selections.is_empty() {
        return connection.clone();
    }

    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();

        match selection.name.as_str() {
            "edges" => {
                let nodes = connection
                    .get("edges")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let edges = nodes
                    .iter()
                    .map(|edge| {
                        let mut projected = Map::new();

                        for edge_selection in &selection.selection {
                            match edge_selection.name.as_str() {
                                "node" => {
                                    let node_type = format!("{object_type}WithRelations");
                                    projected.insert(
                                        edge_selection.response_key().to_string(),
                                        project_record(
                                            edge.get("node").unwrap_or(&Value::Null),
                                            &edge_selection.selection,
                                            &node_type,
                                        ),
                                    );
                                }
                                "cursor" => {
                                    projected.insert(
                                        edge_selection.response_key().to_string(),
                                        edge.get("cursor").cloned().unwrap_or(Value::Null),
                                    );
                                }
                                "__typename" => {
                                    projected.insert(
                                        edge_selection.response_key().to_string(),
                                        Value::String(format!("{object_type}Edge")),
                                    );
                                }
                                _ => {}
                            }
                        }

                        Value::Object(projected)
                    })
                    .collect::<Vec<_>>();

                output.insert(key, Value::Array(edges));
            }
            "pageInfo" => {
                output.insert(
                    key,
                    project_plain(connection.get("pageInfo"), &selection.selection, "PageInfo"),
                );
            }
            "totalCount" => {
                output.insert(
                    key,
                    connection.get("totalCount").cloned().unwrap_or(Value::Null),
                );
            }
            "__typename" => {
                output.insert(key, Value::String(format!("{object_type}Connection")));
            }
            _ => {}
        }
    }

    Value::Object(output)
}

fn project_plain(value: Option<&Value>, selections: &[FieldSelection], type_name: &str) -> Value {
    let value = value.cloned().unwrap_or(Value::Null);

    if selections.is_empty() {
        return value;
    }

    let mut output = Map::new();

    for selection in selections {
        output.insert(
            selection.response_key().to_string(),
            if selection.name == "__typename" {
                Value::String(type_name.to_string())
            } else {
                value.get(&selection.name).cloned().unwrap_or(Value::Null)
            },
        );
    }

    Value::Object(output)
}

fn project_record(record: &Value, selections: &[FieldSelection], type_name: &str) -> Value {
    let mut output = Map::new();

    for selection in selections {
        if selection.name == "__typename" {
            output.insert(
                selection.response_key().to_string(),
                Value::String(type_name.to_string()),
            );
            continue;
        }

        let child = record.get(&selection.name).cloned().unwrap_or(Value::Null);

        output.insert(
            selection.response_key().to_string(),
            if child.is_object() {
                project_record(&child, &selection.selection, selection.name.as_str())
            } else if child.is_array() {
                if selection.selection.is_empty()
                    || child
                        .as_array()
                        .is_some_and(|items| items.iter().any(|item| !item.is_object()))
                {
                    child
                } else {
                    Value::Array(
                        child
                            .as_array()
                            .expect("array checked")
                            .iter()
                            .map(|item| {
                                project_record(item, &selection.selection, selection.name.as_str())
                            })
                            .collect(),
                    )
                }
            } else {
                child
            },
        );
    }

    Value::Object(output)
}

async fn find_duplicates_root(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &Arc<RunnerSchema>,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Result<Value, GraphQlError> {
    let ids = optional_uuid_list_arg(field, "ids")?;
    let data = field
        .argument("data")
        .filter(|value| !value.is_null())
        .cloned();
    let data = match data {
        Some(Value::Array(values)) => Some(values),
        Some(_) => {
            return Err(GraphQlError::user_input(
                "data must be a list of record inputs",
            ))
        }
        None => None,
    };
    match (&ids, &data) {
        (None, None) => {
            return Err(GraphQlError::user_input(
                "You have to provide either \"data\" or \"ids\" argument",
            ))
        }
        (Some(_), Some(_)) => {
            return Err(GraphQlError::user_input(
                "You cannot provide both \"data\" and \"ids\" arguments",
            ))
        }
        (None, Some(values)) if values.is_empty() => {
            return Err(GraphQlError::user_input(
                "The \"data\" condition can not be empty when \"ids\" input not provided",
            ))
        }
        _ => {}
    }
    let connections = find_duplicates(
        &state.core_pool,
        runner_schema.clone(),
        &resolver.object_name_singular,
        ids.as_deref().unwrap_or(&[]),
        data.as_deref(),
        root_record_fields_schemaless(field),
        nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?;
    let object_type = pascal_case(&resolver.object_name_singular);
    Ok(Value::Array(
        connections
            .as_array()
            .into_iter()
            .flatten()
            .map(|connection| project_connection(connection, &field.selection, &object_type))
            .collect(),
    ))
}

async fn merge_many_root(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
    actor: Option<&ActorContext>,
) -> Result<Value, GraphQlError> {
    let ids = field
        .argument("ids")
        .and_then(Value::as_array)
        .ok_or_else(|| GraphQlError::user_input("ids is required"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| GraphQlError::user_input("ids must contain UUIDs"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let conflict_priority_index = field
        .argument("conflictPriorityIndex")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| GraphQlError::user_input("conflictPriorityIndex is required"))?;
    let record = merge_many(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        &MergeManyArgs {
            ids,
            conflict_priority_index,
            dry_run: optional_bool_arg(field, "dryRun"),
        },
        actor,
        root_record_fields_schemaless(field),
        nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?;
    Ok(project_record(
        &record,
        &field.selection,
        &pascal_case(&resolver.object_name_singular),
    ))
}

async fn find_one_root(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Result<Value, GraphQlError> {
    let filter = field
        .argument("filter")
        .cloned()
        .ok_or_else(|| GraphQlError::user_input("Missing filter argument"))?;
    let record = find_one(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        &FindOneArgs {
            filter: Some(filter),
            selected_field_names: root_record_fields_schemaless(field),
            ..Default::default()
        },
        &nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?
    .ok_or_else(record_not_found_graphql_error)?;

    Ok(project_record(
        &record,
        &field.selection,
        &pascal_case(&resolver.object_name_singular),
    ))
}

#[allow(clippy::too_many_arguments)]
async fn mutate_existing_record(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
    kind: FilteredMutationKind,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
) -> Result<Value, GraphQlError> {
    let id = required_id(field)?;
    let snapshot = run_mutation_one_with_snapshot(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        kind,
        Some(id_filter_value(id)),
        data,
        actor,
        with_deleted,
        Vec::new(),
    )
    .await
    .map_err(runner_graphql_error)?;
    let action = if kind == FilteredMutationKind::Update {
        "updated"
    } else {
        "restored"
    };
    if let Some(properties) =
        crate::events::record_mutation_properties(&snapshot.before, snapshot.after.as_ref())
    {
        let workspace_id = runner_schema
            .object(&resolver.object_name_singular)
            .map_err(runner_graphql_error)?
            .object
            .workspace_id;
        state.event_streams.publish_record(
            workspace_id,
            action,
            &resolver.object_name_singular,
            &id.to_string(),
            properties,
            runner_schema,
            actor,
        );
    }

    find_record(
        state,
        schema,
        runner_schema,
        field,
        resolver,
        id,
        with_deleted,
    )
    .await
}

async fn find_record(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
    id: Uuid,
    with_deleted: bool,
) -> Result<Value, GraphQlError> {
    let result = find_many(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        &FindManyArgs {
            filter: Some(id_filter_value(id)),
            with_deleted,
            selected_field_names: root_record_fields_schemaless(field),
            ..Default::default()
        },
        false,
        &nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?;
    let record = result
        .records
        .into_iter()
        .next()
        .ok_or_else(record_not_found_graphql_error)?;

    Ok(project_record(
        &record,
        &field.selection,
        &pascal_case(&resolver.object_name_singular),
    ))
}

fn root_record_fields(
    schema: &WorkspaceSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Vec<String> {
    let metadata_fields = schema.fields(&resolver.object_name_singular);
    let mut names = Vec::new();

    for selection in node_selection(field) {
        if selection.name == "__typename" {
            continue;
        }
        let Some(metadata) = metadata_fields
            .iter()
            .find(|metadata| metadata.name == selection.name)
        else {
            names.push(selection.name.clone());
            continue;
        };
        if matches!(
            metadata.field_type,
            FieldMetadataType::Relation | FieldMetadataType::MorphRelation
        ) {
            continue;
        }
        if crate::workspace_migrations::column_defs::composite_types::is_composite_field_metadata_type(
            crate::workspace_migrations::runner::ddl_field_type(metadata.field_type),
        ) {
            for sub_field in &selection.selection {
                if sub_field.name != "__typename" {
                    names.push(compute_composite_column_name(
                        &metadata.name,
                        &sub_field.name,
                    ));
                }
            }
        } else {
            names.push(selection.name.clone());
        }
    }

    if !names.iter().any(|name| name == "id") {
        names.insert(0, String::from("id"));
    }
    names
}

fn root_record_fields_schemaless(field: &FieldSelection) -> Vec<String> {
    let record_selection = node_selection(field);
    let mut names = record_selection
        .iter()
        .filter(|selection| selection.name != "__typename")
        .map(|selection| selection.name.clone())
        .collect::<Vec<_>>();

    if !names.iter().any(|name| name == "id") {
        names.insert(0, String::from("id"));
    }

    names
}

fn nested_relations(
    schema: &WorkspaceSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Vec<NestedRelationSelection> {
    let record_selection = node_selection(field);
    let mut relations = Vec::new();

    for selection in record_selection {
        let Some(relation) = schema
            .fields(&resolver.object_name_singular)
            .iter()
            .find(|metadata| metadata.name == selection.name)
        else {
            continue;
        };

        if !matches!(
            relation.field_type,
            FieldMetadataType::Relation | FieldMetadataType::MorphRelation
        ) {
            continue;
        }

        let mut selected = selection
            .selection
            .iter()
            .map(|nested| nested.name.clone())
            .filter(|name| name != "__typename")
            .collect::<Vec<_>>();

        if !selected.iter().any(|name| name == "id") {
            selected.insert(0, String::from("id"));
        }

        relations.push(NestedRelationSelection {
            relation_field_name: selection.name.clone(),
            selected_field_names: selected,
            first: selection
                .argument("first")
                .and_then(Value::as_u64)
                .map(|value| value as usize),
        });
    }

    relations
}

fn node_selection(field: &FieldSelection) -> &[FieldSelection] {
    field
        .selection
        .iter()
        .find(|selection| selection.name == "edges")
        .and_then(|edges| {
            edges
                .selection
                .iter()
                .find(|selection| selection.name == "node")
        })
        .map(|node| node.selection.as_slice())
        .unwrap_or(field.selection.as_slice())
}

fn required_object(field: &FieldSelection, name: &str) -> Result<Map<String, Value>, GraphQlError> {
    field
        .argument(name)
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| GraphQlError::user_input(format!("{name} is required")))
}

fn required_id(field: &FieldSelection) -> Result<Uuid, GraphQlError> {
    field
        .argument("id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| GraphQlError::user_input("id is required"))
}

fn optional_uuid_list_arg(
    field: &FieldSelection,
    name: &str,
) -> Result<Option<Vec<Uuid>>, GraphQlError> {
    let Some(value) = field.argument(name).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .ok_or_else(|| GraphQlError::user_input(format!("{name} must be a list of UUIDs")))?;
    values
        .iter()
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_str()
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| GraphQlError::user_input(format!("{name} must be a list of UUIDs")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn optional_bool_arg(field: &FieldSelection, name: &str) -> bool {
    field
        .argument(name)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn optional_string_arg(field: &FieldSelection, name: &str) -> Option<String> {
    field
        .argument(name)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn optional_usize(field: &FieldSelection, name: &str) -> Option<usize> {
    field
        .argument(name)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
}

fn id_filter(id: Uuid) -> Value {
    id_filter_value(id)
}

fn id_filter_value(id: Uuid) -> Value {
    Value::Object(single_entry(
        "id",
        Value::Object(single_entry("eq", Value::String(id.to_string()))),
    ))
}

fn single_entry(key: &str, value: Value) -> Map<String, Value> {
    Map::from_iter([(key.to_string(), value)])
}

async fn core_operation(
    state: &AppState,
    _workspace_id: Uuid,
    auth: &AuthContext,
    field: &FieldSelection,
) -> Result<Value, GraphQlError> {
    match field.name.as_str() {
        "currentUser" => current_user(state, auth, &field.selection).await,
        "currentWorkspace" => current_workspace(state, auth, &field.selection).await,
        "__typename" => Ok(Value::String(String::from("Query"))),
        _ => Err(GraphQlError::user_input(format!(
            "Core field \"{}\" is not implemented by this endpoint",
            field.name
        ))),
    }
}

async fn current_user(
    state: &AppState,
    auth: &AuthContext,
    selection: &[FieldSelection],
) -> Result<Value, GraphQlError> {
    let user_id = auth.user_id.ok_or_else(|| {
        GraphQlError::unauthenticated("Authentication required").with_sub_code("UNAUTHENTICATED")
    })?;
    let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, bool)>(
        "SELECT id, email, \"firstName\", \"lastName\", \"isEmailVerified\" \
         FROM core.\"user\" WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?
    .ok_or_else(|| GraphQlError::not_found("User was not found").with_sub_code("USER_NOT_FOUND"))?;
    let current_user_workspace = current_user_workspace_permissions_for_auth(state, auth)
        .await
        .map_err(GraphQlError::from)?;
    let mut record = json!({
        "id": row.0,
        "email": row.1,
        "firstName": row.2,
        "lastName": row.3,
        "isEmailVerified": row.4,
        "__typename": "User",
    });
    if let Some(workspace) = current_user_workspace {
        record["currentUserWorkspace"] = workspace;
    }

    Ok(project_record(&record, selection, "User"))
}

async fn current_workspace(
    state: &AppState,
    auth: &AuthContext,
    selection: &[FieldSelection],
) -> Result<Value, GraphQlError> {
    let workspace_id = auth.workspace_id.ok_or_else(|| {
        GraphQlError::unauthenticated("Authentication required").with_sub_code("UNAUTHENTICATED")
    })?;
    let row = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, \"displayName\", subdomain FROM core.\"workspace\" WHERE id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?
    .ok_or_else(|| {
        GraphQlError::not_found("Workspace was not found").with_sub_code("WORKSPACE_NOT_FOUND")
    })?;
    let record = json!({
        "id": row.0,
        "displayName": row.1,
        "subdomain": row.2,
        "__typename": "Workspace",
    });

    Ok(project_record(&record, selection, "Workspace"))
}

fn schema_type(kind: &str, name: &str) -> Value {
    json!({ "kind": kind, "name": name })
}

fn named_type(name: &str) -> Value {
    schema_type("OBJECT", name)
}

fn non_null(inner: Value) -> Value {
    json!({
        "kind": "NON_NULL",
        "name": Value::Null,
        "ofType": inner,
    })
}

fn list(inner: Value) -> Value {
    json!({
        "kind": "LIST",
        "name": Value::Null,
        "ofType": inner,
    })
}

fn scalar_type(name: &str) -> Value {
    schema_type("SCALAR", name)
}

fn field_entry(name: &str, type_value: Value) -> Value {
    json!({
        "name": name,
        "type": type_value,
        "args": [],
    })
}

fn input_value_entry(name: &str, type_value: Value) -> Value {
    json!({
        "name": name,
        "description": Value::Null,
        "defaultValue": Value::Null,
        "type": type_value,
    })
}

fn schema_type_specs(schema: &WorkspaceSchema) -> Vec<Value> {
    let mut specs = vec![
        field_entry("queryType", non_null(named_type("Query"))),
        field_entry("mutationType", named_type("Mutation")),
        field_entry("subscriptionType", Value::Null),
        field_entry("types", list(non_null(named_type("__Type")))),
        field_entry("directives", list(non_null(named_type("__Directive")))),
    ];

    specs.push(json!({
        "kind": "OBJECT",
        "name": "PageInfo",
        "fields": [
            field_entry("startCursor", scalar_type("String")),
            field_entry("endCursor", scalar_type("String")),
            field_entry("hasNextPage", non_null(scalar_type("Boolean"))),
            field_entry("hasPreviousPage", non_null(scalar_type("Boolean"))),
        ],
    }));

    for name in ["DateTime", "JSON", "RawJson", "UUID"] {
        specs.push(json!({
            "kind": "SCALAR",
            "name": name,
        }));
    }

    let uuid_filter_fields = vec![
        input_value_entry("eq", scalar_type("UUID")),
        input_value_entry("gt", scalar_type("UUID")),
        input_value_entry("gte", scalar_type("UUID")),
        input_value_entry("in", list(scalar_type("UUID"))),
        input_value_entry("lt", scalar_type("UUID")),
        input_value_entry("lte", scalar_type("UUID")),
        input_value_entry("neq", scalar_type("UUID")),
        input_value_entry("is", schema_type("ENUM", "FilterIs")),
    ];
    specs.push(json!({
        "kind": "INPUT_OBJECT",
        "name": "UUIDFilter",
        "inputFields": uuid_filter_fields,
    }));

    let date_time_filter_fields = vec![
        input_value_entry("eq", scalar_type("DateTime")),
        input_value_entry("gt", scalar_type("DateTime")),
        input_value_entry("gte", scalar_type("DateTime")),
        input_value_entry("in", list(non_null(scalar_type("DateTime")))),
        input_value_entry("lt", scalar_type("DateTime")),
        input_value_entry("lte", scalar_type("DateTime")),
        input_value_entry("neq", scalar_type("DateTime")),
        input_value_entry("is", schema_type("ENUM", "FilterIs")),
        input_value_entry("gtField", scalar_type("String")),
        input_value_entry("gteField", scalar_type("String")),
        input_value_entry("ltField", scalar_type("String")),
        input_value_entry("lteField", scalar_type("String")),
    ];
    specs.push(json!({
        "kind": "INPUT_OBJECT",
        "name": "DateTimeFilter",
        "inputFields": date_time_filter_fields,
    }));
    specs.push(json!({
        "kind": "INPUT_OBJECT",
        "name": "ObjectRecordFilterInput",
        "inputFields": [
            input_value_entry(
                "and",
                list(schema_type("INPUT_OBJECT", "ObjectRecordFilterInput")),
            ),
            input_value_entry("not", schema_type("INPUT_OBJECT", "ObjectRecordFilterInput")),
            input_value_entry(
                "or",
                list(schema_type("INPUT_OBJECT", "ObjectRecordFilterInput")),
            ),
            input_value_entry("id", schema_type("INPUT_OBJECT", "UUIDFilter")),
            input_value_entry("createdAt", schema_type("INPUT_OBJECT", "DateTimeFilter")),
            input_value_entry("updatedAt", schema_type("INPUT_OBJECT", "DateTimeFilter")),
            input_value_entry("deletedAt", schema_type("INPUT_OBJECT", "DateTimeFilter")),
        ],
    }));
    specs.push(json!({
        "kind": "ENUM",
        "name": "FilterIs",
        "enumValues": [
            {"name": "NULL"},
            {"name": "NOT_NULL"},
        ],
    }));

    specs.push(json!({
        "kind": "OBJECT",
        "name": "SearchRecord",
        "fields": [
            field_entry("recordId", non_null(scalar_type("UUID"))),
            field_entry("objectNameSingular", non_null(scalar_type("String"))),
            field_entry("objectLabelSingular", non_null(scalar_type("String"))),
            field_entry("label", non_null(scalar_type("String"))),
            field_entry("imageUrl", scalar_type("String")),
            field_entry("tsRankCD", non_null(scalar_type("Float"))),
            field_entry("tsRank", non_null(scalar_type("Float"))),
        ],
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "SearchResultEdge",
        "fields": [
            field_entry("node", non_null(named_type("SearchRecord"))),
            field_entry("cursor", non_null(scalar_type("String"))),
        ],
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "SearchResultPageInfo",
        "fields": [
            field_entry("endCursor", scalar_type("String")),
            field_entry("hasNextPage", non_null(scalar_type("Boolean"))),
        ],
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "SearchResultConnection",
        "fields": [
            field_entry("edges", non_null(list(non_null(named_type("SearchResultEdge"))))),
            field_entry("pageInfo", non_null(named_type("SearchResultPageInfo"))),
        ],
    }));

    let mut search_field = field_entry("search", non_null(named_type("SearchResultConnection")));
    search_field["args"] = Value::Array(vec![
        input_value_entry("searchInput", non_null(scalar_type("String"))),
        input_value_entry("limit", non_null(scalar_type("Int"))),
        input_value_entry("after", scalar_type("String")),
        input_value_entry(
            "includedObjectNameSingulars",
            list(non_null(scalar_type("String"))),
        ),
        input_value_entry(
            "filter",
            schema_type("INPUT_OBJECT", "ObjectRecordFilterInput"),
        ),
        input_value_entry(
            "excludedObjectNameSingulars",
            list(non_null(scalar_type("String"))),
        ),
    ]);
    let mut query_fields = vec![
        field_entry("currentUser", non_null(named_type("User"))),
        field_entry("currentWorkspace", non_null(named_type("Workspace"))),
        search_field,
    ];
    let mut mutation_fields = Vec::new();

    for object in schema.objects() {
        let singular = camel_case(&object.name_singular);
        let plural = camel_case(&object.name_plural);
        let type_name = pascal_case(&object.name_singular);
        let connection_name = format!("{type_name}Connection");

        query_fields.push(field_entry(&plural, non_null(named_type(&connection_name))));
        query_fields.push(field_entry(&singular, non_null(named_type(&type_name))));
        if object.duplicate_criteria.is_some() {
            let mut duplicates = field_entry(
                &format!("{singular}Duplicates"),
                list(non_null(named_type(&connection_name))),
            );
            duplicates["args"] = Value::Array(vec![
                input_value_entry("ids", list(scalar_type("UUID"))),
                input_value_entry(
                    "data",
                    list(schema_type(
                        "INPUT_OBJECT",
                        &format!("{type_name}CreateInput"),
                    )),
                ),
            ]);
            query_fields.push(duplicates);
        }

        let singular_pascal = pascal_case(&object.name_singular);
        let plural_pascal = pascal_case(&object.name_plural);

        for prefix in ["create", "update", "delete", "destroy", "restore"] {
            mutation_fields.push(field_entry(
                &format!("{prefix}{singular_pascal}"),
                non_null(named_type(&type_name)),
            ));
        }

        mutation_fields.push(field_entry(
            &format!("create{plural_pascal}"),
            non_null(list(non_null(named_type(&type_name)))),
        ));
        let mut merge = field_entry(
            &format!("merge{plural_pascal}"),
            non_null(named_type(&type_name)),
        );
        merge["args"] = Value::Array(vec![
            input_value_entry("ids", non_null(list(non_null(scalar_type("UUID"))))),
            input_value_entry("conflictPriorityIndex", non_null(scalar_type("Int"))),
            input_value_entry("dryRun", scalar_type("Boolean")),
        ]);
        merge["args"][2]["defaultValue"] = Value::String(String::from("false"));
        mutation_fields.push(merge);
    }

    specs.push(json!({
        "kind": "OBJECT",
        "name": "ObjectRecordsPermissions",
        "fields": [
            field_entry("objectMetadataId", non_null(scalar_type("UUID"))),
            field_entry("canCreateAllRecords", non_null(scalar_type("Boolean"))),
            field_entry("canReadAllRecords", non_null(scalar_type("Boolean"))),
            field_entry("canUpdateAllRecords", non_null(scalar_type("Boolean"))),
            field_entry("canSoftDeleteAllRecords", non_null(scalar_type("Boolean"))),
            field_entry("canDestroyAllRecords", non_null(scalar_type("Boolean"))),
            field_entry("unreadableFields", non_null(list(scalar_type("UUID")))),
            field_entry("uneditableFields", non_null(list(scalar_type("UUID")))),
            field_entry("rowLevelRecordsPermissions", list(named_type("RowLevelRecordsPermissions"))),
            field_entry("canReadSomeRecords", non_null(scalar_type("Boolean"))),
        ],
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "RowLevelRecordsPermissions",
        "fields": [
            field_entry("ruleId", non_null(scalar_type("UUID"))),
            field_entry("canCreateRecords", non_null(scalar_type("Boolean"))),
            field_entry("canReadRecords", non_null(scalar_type("Boolean"))),
            field_entry("canUpdateRecords", non_null(scalar_type("Boolean"))),
            field_entry("canSoftDeleteRecords", non_null(scalar_type("Boolean"))),
            field_entry("canDestroyRecords", non_null(scalar_type("Boolean"))),
            field_entry("predicates", non_null(list(scalar_type("JSON")))),
            field_entry("predicateGroups", non_null(list(scalar_type("JSON")))),
            field_entry("unreadableFields", non_null(list(scalar_type("UUID")))),
            field_entry("uneditableFields", non_null(list(scalar_type("UUID")))),
        ],
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "UserWorkspace",
        "fields": [
            field_entry("id", non_null(scalar_type("UUID"))),
            field_entry("permissionFlags", list(scalar_type("String"))),
            field_entry("objectRecordsPermissions", list(non_null(named_type("ObjectRecordsPermissions")))),
        ],
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "User",
        "fields": [
            field_entry("id", non_null(scalar_type("UUID"))),
            field_entry("firstName", non_null(scalar_type("String"))),
            field_entry("lastName", non_null(scalar_type("String"))),
            field_entry("email", non_null(scalar_type("String"))),
            field_entry("isEmailVerified", non_null(scalar_type("Boolean"))),
            field_entry("currentUserWorkspace", named_type("UserWorkspace")),
        ],
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "Query",
        "fields": query_fields,
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "Mutation",
        "fields": mutation_fields,
    }));

    for object in schema.objects() {
        let type_name = pascal_case(&object.name_singular);
        let mut fields = vec![field_entry("id", non_null(scalar_type("ID")))];

        for field in schema.fields(&object.name_singular) {
            let output_type = match field.field_type {
                FieldMetadataType::Relation => {
                    let relation_type = field
                        .settings
                        .as_ref()
                        .and_then(|settings| settings.get("relationType"))
                        .and_then(Value::as_str);

                    if relation_type == Some("ONE_TO_MANY") {
                        let target = relation_target(schema, field);
                        list(non_null(named_type(&format!("{target}Connection"))))
                    } else {
                        named_type(&relation_target(schema, field))
                    }
                }
                FieldMetadataType::Uuid => non_null(scalar_type("ID")),
                FieldMetadataType::Boolean => non_null(scalar_type("Boolean")),
                FieldMetadataType::Number
                | FieldMetadataType::Numeric
                | FieldMetadataType::Rating
                | FieldMetadataType::Position => scalar_type("Float"),
                FieldMetadataType::Array | FieldMetadataType::MultiSelect => {
                    list(non_null(scalar_type("String")))
                }
                _ => scalar_type(graphql_scalar_name(field.field_type)),
            };

            fields.push(field_entry(&field.name, output_type));
        }

        specs.push(json!({
            "kind": "OBJECT",
            "name": type_name,
            "fields": fields,
        }));

        let edge_name = format!("{type_name}Edge");
        let connection_name = format!("{type_name}Connection");
        specs.push(json!({
            "kind": "OBJECT",
            "name": edge_name,
            "fields": [
                field_entry("node", non_null(named_type(&type_name))),
                field_entry("cursor", non_null(scalar_type("String"))),
            ],
        }));
        specs.push(json!({
            "kind": "OBJECT",
            "name": connection_name,
            "fields": [
                field_entry("edges", list(non_null(named_type(&edge_name)))),
                field_entry("pageInfo", non_null(named_type("PageInfo"))),
                field_entry("totalCount", scalar_type("Int")),
            ],
        }));
    }

    specs
}

fn relation_target(
    schema: &WorkspaceSchema,
    field: &crate::metadata::flat::FlatFieldMetadata,
) -> String {
    field
        .relation_target_object_metadata_id
        .and_then(|target_id| {
            schema
                .objects()
                .iter()
                .find(|object| object.id == target_id)
        })
        .map(|object| pascal_case(&object.name_singular))
        .unwrap_or_else(|| String::from("JSON"))
}

fn graphql_scalar_name(field_type: FieldMetadataType) -> &'static str {
    match field_type {
        FieldMetadataType::Uuid => "ID",
        FieldMetadataType::Boolean => "Boolean",
        FieldMetadataType::Number
        | FieldMetadataType::Numeric
        | FieldMetadataType::Rating
        | FieldMetadataType::Position => "Float",
        FieldMetadataType::Date => "Date",
        FieldMetadataType::DateTime => "DateTime",
        FieldMetadataType::RawJson | FieldMetadataType::Files => "RawJson",
        _ => "String",
    }
}

fn introspection_value(field: &FieldSelection, schema: &WorkspaceSchema) -> Value {
    match field.name.as_str() {
        "__schema" => introspect_schema(&field.selection, schema),
        "__type" => {
            let name = field
                .argument("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            introspect_type(
                schema_type_specs(schema)
                    .into_iter()
                    .find(|spec| spec.get("name").and_then(Value::as_str) == Some(name))
                    .as_ref(),
                &field.selection,
            )
        }
        _ => Value::Null,
    }
}

fn introspect_schema(selections: &[FieldSelection], schema: &WorkspaceSchema) -> Value {
    let specs = schema_type_specs(schema);
    let mut output = Map::new();

    for selection in selections {
        let value = match selection.name.as_str() {
            "queryType" => introspect_type(
                specs
                    .iter()
                    .find(|spec| spec.get("name").and_then(Value::as_str) == Some("Query")),
                &selection.selection,
            ),
            "mutationType" => introspect_type(
                specs
                    .iter()
                    .find(|spec| spec.get("name").and_then(Value::as_str) == Some("Mutation")),
                &selection.selection,
            ),
            "subscriptionType" => Value::Null,
            "types" => Value::Array(
                specs
                    .iter()
                    .map(|spec| introspect_type(Some(spec), &selection.selection))
                    .collect(),
            ),
            "directives" => Value::Array(vec![]),
            _ => Value::Null,
        };

        output.insert(selection.response_key().to_string(), value);
    }

    Value::Object(output)
}

fn introspect_type(spec: Option<&Value>, selections: &[FieldSelection]) -> Value {
    let Some(spec) = spec else {
        return Value::Null;
    };

    if selections.is_empty() {
        return Value::Object(
            spec.get("name")
                .and_then(Value::as_str)
                .map(|name| {
                    Map::from_iter([(String::from("name"), Value::String(name.to_string()))])
                })
                .unwrap_or_default(),
        );
    }

    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();
        let value = match selection.name.as_str() {
            "kind" => spec.get("kind").cloned().unwrap_or(Value::Null),
            "name" => spec.get("name").cloned().unwrap_or(Value::Null),
            "description" => Value::Null,
            "fields" => spec
                .get("fields")
                .and_then(Value::as_array)
                .map(|fields| {
                    fields
                        .iter()
                        .map(|field| introspect_field(field, &selection.selection))
                        .collect::<Vec<_>>()
                })
                .map(Value::Array)
                .unwrap_or(Value::Null),
            "args" => Value::Array(vec![]),
            "interfaces" => Value::Array(vec![]),
            "possibleTypes" => Value::Null,
            "enumValues" => spec
                .get("enumValues")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .map(|value| introspect_enum_value(value, &selection.selection))
                        .collect::<Vec<_>>()
                })
                .map(Value::Array)
                .unwrap_or(Value::Null),
            "inputFields" => spec
                .get("inputFields")
                .and_then(Value::as_array)
                .map(|fields| {
                    fields
                        .iter()
                        .map(|field| introspect_input_value(field, &selection.selection))
                        .collect::<Vec<_>>()
                })
                .map(Value::Array)
                .unwrap_or(Value::Null),
            "ofType" => spec
                .get("ofType")
                .map(|inner| introspect_type(Some(inner), &selection.selection))
                .unwrap_or(Value::Null),
            _ => Value::Null,
        };

        output.insert(key, value);
    }

    Value::Object(output)
}

fn introspect_field(field: &Value, selections: &[FieldSelection]) -> Value {
    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();
        let value = match selection.name.as_str() {
            "name" => field.get("name").cloned().unwrap_or(Value::Null),
            "description" => Value::Null,
            "type" => field
                .get("type")
                .map(|type_value| introspect_type(Some(type_value), &selection.selection))
                .unwrap_or(Value::Null),
            "args" => field
                .get("args")
                .and_then(Value::as_array)
                .map(|args| {
                    args.iter()
                        .map(|arg| introspect_input_value(arg, &selection.selection))
                        .collect::<Vec<_>>()
                })
                .map(Value::Array)
                .unwrap_or_else(|| Value::Array(vec![])),
            "isDeprecated" => Value::Bool(false),
            "deprecationReason" => Value::Null,
            _ => Value::Null,
        };

        output.insert(key, value);
    }

    Value::Object(output)
}

fn introspect_input_value(value: &Value, selections: &[FieldSelection]) -> Value {
    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();
        let value = match selection.name.as_str() {
            "name" => value.get("name").cloned().unwrap_or(Value::Null),
            "description" => Value::Null,
            "defaultValue" => value.get("defaultValue").cloned().unwrap_or(Value::Null),
            "type" => value
                .get("type")
                .map(|type_value| introspect_type(Some(type_value), &selection.selection))
                .unwrap_or(Value::Null),
            _ => Value::Null,
        };

        output.insert(key, value);
    }

    Value::Object(output)
}

fn introspect_enum_value(value: &Value, selections: &[FieldSelection]) -> Value {
    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();
        let value = match selection.name.as_str() {
            "name" => value.get("name").cloned().unwrap_or(Value::Null),
            "description" => Value::Null,
            "isDeprecated" => Value::Bool(false),
            "deprecationReason" => Value::Null,
            _ => Value::Null,
        };

        output.insert(key, value);
    }

    Value::Object(output)
}

fn unused_schema_helpers(schema: &WorkspaceSchema) -> HashMap<String, Vec<String>> {
    HashMap::from_iter([(String::from("query"), schema.query_root_fields())])
}

fn unused_camel_marker(value: &str) -> String {
    camel_case(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::flat::FlatFieldMetadata;
    use crate::metadata::flat::FlatObjectMetadata;
    use crate::metadata::flat::FlatWorkspaceMaps;

    #[test]
    fn search_locale_header_normalization_matches_upstream() {
        assert_eq!(normalize_locale(None), "en");
        assert_eq!(normalize_locale(Some("fr")), "fr-FR");
        assert_eq!(normalize_locale(Some("FR-fr")), "fr-FR");
        assert_eq!(normalize_locale(Some(" fr ")), "fr-FR");
        assert_eq!(normalize_locale(Some("pt")), "pt-BR");
        assert_eq!(normalize_locale(Some("sr")), "sr-Cyrl");
        assert_eq!(normalize_locale(Some("zh-HK")), "zh-CN");
        assert_eq!(normalize_locale(Some("unknown")), "en");

        let mut headers = HeaderMap::new();
        headers.insert("x-locale", "vi".parse().expect("locale header"));
        assert_eq!(
            normalize_locale(headers.get("x-locale").and_then(|v| v.to_str().ok())),
            "vi-VN"
        );
    }

    #[test]
    fn search_introspection_exposes_the_upstream_filter_contract() {
        let schema = WorkspaceSchema::build(&FlatWorkspaceMaps::default());
        let specs = schema_type_specs(&schema);
        for name in ["DateTime", "JSON", "RawJson", "UUID"] {
            assert!(specs
                .iter()
                .any(|spec| { spec["kind"] == "SCALAR" && spec["name"] == name }));
        }
        let filter = specs
            .iter()
            .find(|spec| spec["name"] == "ObjectRecordFilterInput")
            .expect("ObjectRecordFilterInput");
        let fields = filter["inputFields"].as_array().expect("input fields");
        let names = fields
            .iter()
            .map(|field| field["name"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();

        assert_eq!(filter["kind"], "INPUT_OBJECT");
        assert_eq!(
            names,
            [
                "and",
                "not",
                "or",
                "id",
                "createdAt",
                "updatedAt",
                "deletedAt"
            ]
        );
        assert_eq!(fields[0]["type"]["kind"], "LIST");
        assert_eq!(
            fields[0]["type"]["ofType"]["name"],
            "ObjectRecordFilterInput"
        );

        let date_time = specs
            .iter()
            .find(|spec| spec["name"] == "DateTimeFilter")
            .expect("DateTimeFilter");
        assert_eq!(
            date_time["inputFields"][3]["type"]["ofType"]["kind"],
            "NON_NULL"
        );
        assert_eq!(date_time["inputFields"][8]["name"], "gtField");
        assert_eq!(date_time["inputFields"][11]["name"], "lteField");

        let filter_is = specs
            .iter()
            .find(|spec| spec["name"] == "FilterIs")
            .expect("FilterIs");
        assert_eq!(filter_is["kind"], "ENUM");
        assert_eq!(filter_is["enumValues"][0]["name"], "NULL");
        assert_eq!(filter_is["enumValues"][1]["name"], "NOT_NULL");

        let query = specs
            .iter()
            .find(|spec| spec["name"] == "Query")
            .expect("Query");
        let search = query["fields"]
            .as_array()
            .expect("query fields")
            .iter()
            .find(|field| field["name"] == "search")
            .expect("search");
        let args = search["args"].as_array().expect("search args");
        let names = args
            .iter()
            .map(|arg| arg["name"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            [
                "searchInput",
                "limit",
                "after",
                "includedObjectNameSingulars",
                "filter",
                "excludedObjectNameSingulars",
            ]
        );
        assert_eq!(args[0]["type"]["kind"], "NON_NULL");
        assert_eq!(args[1]["type"]["kind"], "NON_NULL");
        assert_eq!(args[4]["type"]["kind"], "INPUT_OBJECT");
        assert_eq!(args[4]["type"]["name"], "ObjectRecordFilterInput");
    }

    #[test]
    fn search_filter_validation_matches_the_upstream_input_type() {
        let id = Uuid::new_v4().to_string();
        assert!(validate_search_filter(&json!({
            "and": [{"id": {"eq": id}}],
            "or": [{"deletedAt": {"is": "NULL"}}],
            "not": {"updatedAt": {"gte": "2026-11-28T00:00:00Z"}},
            "id": {"in": [id, null], "neq": Uuid::nil().to_string()},
            "createdAt": {"gtField": "updatedAt", "lteField": "createdAt"}
        }))
        .is_ok());
        assert!(validate_search_filter(&json!({"name": {"eq": "Ada"}})).is_err());
        assert!(validate_search_filter(&json!({"id": {"eq": "not-a-uuid"}})).is_err());
        assert!(validate_search_filter(&json!({"createdAt": {"eq": "yesterday"}})).is_err());
        assert!(validate_search_filter(&json!({"id": {"is": "EMPTY"}})).is_err());
        assert!(validate_search_filter(&json!({"and": {"id": {"eq": id}}})).is_err());
    }

    #[test]
    fn mutation_returning_fields_flatten_composite_selections() {
        let workspace_id = Uuid::new_v4();
        let object_id = Uuid::new_v4();
        let fields = vec![
            FlatFieldMetadata {
                id: Uuid::new_v4(),
                object_metadata_id: object_id,
                field_type: FieldMetadataType::Uuid,
                name: String::from("id"),
                label: String::from("id"),
                ..Default::default()
            },
            FlatFieldMetadata {
                id: Uuid::new_v4(),
                object_metadata_id: object_id,
                field_type: FieldMetadataType::Links,
                name: String::from("domainName"),
                label: String::from("domainName"),
                ..Default::default()
            },
        ];
        let mut maps = FlatWorkspaceMaps::default();
        for field in &fields {
            maps.field_metadata_maps.add(
                field.id.to_string(),
                field.clone(),
                field.id.to_string(),
                workspace_id.to_string(),
            );
        }
        let object = FlatObjectMetadata {
            id: object_id,
            workspace_id,
            name_singular: String::from("company"),
            name_plural: String::from("companies"),
            field_ids: fields.iter().map(|field| field.id.to_string()).collect(),
            ..Default::default()
        };
        maps.object_metadata_maps.add(
            object.id.to_string(),
            object.clone(),
            object.universal_identifier.clone(),
            workspace_id.to_string(),
        );
        let schema = WorkspaceSchema::build(&maps);
        let resolver = super::super::schema::RootResolver {
            method: super::super::schema::ResolverMethod::UpdateOne,
            object_name_singular: String::from("company"),
        };
        let selection = |name: &str, selection: Vec<FieldSelection>| FieldSelection {
            alias: None,
            name: name.to_string(),
            line: 1,
            column: 1,
            arguments: Vec::new(),
            selection,
            spreads: Vec::new(),
        };
        let field = selection(
            "updateCompany",
            vec![selection(
                "domainName",
                vec![selection("primaryLinkUrl", Vec::new())],
            )],
        );

        assert_eq!(
            root_record_fields(&schema, &field, &resolver),
            ["id", "domainNamePrimaryLinkUrl"]
        );
    }
}
