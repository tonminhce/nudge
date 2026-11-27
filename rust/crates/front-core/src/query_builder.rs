use crate::projection::project_node;
use crate::projection::ObjectMetadata;
use crate::projection::ProjectionOptions;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQLDocument {
    pub query: String,
    pub variables: Map<String, Value>,
}

impl GraphQLDocument {
    fn new(query: String, variables: Map<String, Value>) -> Self {
        Self { query, variables }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueryBuildError {
    #[error("invalid GraphQL object name: {0}")]
    InvalidObjectName(String),
}

#[derive(Debug, Clone, Default)]
pub struct FindManyArgs {
    pub filter: Option<Value>,
    pub order_by: Option<Value>,
    pub first: Option<usize>,
    pub last: Option<usize>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub offset: Option<i64>,
    pub depth: usize,
    pub include_total_count: Option<bool>,
}

pub fn find_many_query(
    object: &ObjectMetadata,
    args: FindManyArgs,
) -> Result<GraphQLDocument, QueryBuildError> {
    validate_object_name(&object.name_singular)?;
    validate_object_name(&object.name_plural)?;

    let type_name = pascal_case(&object.name_singular);
    let operation = format!("FindMany{}", pascal_case(&object.name_plural));
    let projection = project_node(
        object,
        ProjectionOptions {
            depth: args.depth,
            include_system_fields: true,
        },
    );
    let mut variables = Map::new();
    let mut arguments = Vec::new();

    if let Some(filter) = args.filter {
        arguments.push(String::from("filter: $filter"));
        variables.insert(String::from("filter"), filter);
    }

    if let Some(order_by) = args.order_by {
        arguments.push(String::from("orderBy: $orderBy"));
        variables.insert(String::from("orderBy"), order_by);
    }

    if let Some(first) = args.first.or_else(|| args.last.is_none().then_some(60)) {
        arguments.push(String::from("first: $first"));
        variables.insert(String::from("first"), Value::from(first));
    }

    if let Some(last) = args.last {
        arguments.push(String::from("last: $last"));
        variables.insert(String::from("last"), Value::from(last));
    }

    if let Some(after) = args.after {
        arguments.push(String::from("after: $after"));
        variables.insert(String::from("after"), Value::String(after));
    }

    if let Some(before) = args.before {
        arguments.push(String::from("before: $before"));
        variables.insert(String::from("before"), Value::String(before));
    }

    if let Some(offset) = args.offset {
        arguments.push(String::from("offset: $offset"));
        variables.insert(String::from("offset"), Value::from(offset));
    }

    let variable_definitions = variables
        .iter()
        .map(|(name, value)| format!("${name}: {}", variable_type(name, value, &type_name)))
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = arguments.join(", ");
    let total_count = if args.include_total_count.unwrap_or(true) {
        " totalCount"
    } else {
        ""
    };
    let query = format!(
        "query {operation}({variable_definitions}) {{ {}({arguments}) {{ edges {{ node {{ {projection} }} cursor }} pageInfo {{ hasNextPage hasPreviousPage startCursor endCursor }}{total_count} }} }}",
        object.name_plural
    );

    Ok(GraphQLDocument::new(query, variables))
}

pub fn create_one_mutation(
    object: &ObjectMetadata,
    data: Value,
) -> Result<GraphQLDocument, QueryBuildError> {
    validate_object_name(&object.name_singular)?;
    let singular = pascal_case(&object.name_singular);
    let type_name = format!("{}CreateInput", singular);
    let projection = project_node(object, ProjectionOptions::default());
    let mut variables = Map::new();
    variables.insert(String::from("data"), data);
    let query = format!(
        "mutation CreateOne{singular}($data: {type_name}!) {{ create{singular}(data: $data) {{ {projection} }} }}"
    );
    Ok(GraphQLDocument::new(query, variables))
}

pub fn update_one_mutation(
    object: &ObjectMetadata,
    id: String,
    data: Value,
) -> Result<GraphQLDocument, QueryBuildError> {
    validate_object_name(&object.name_singular)?;
    let singular = pascal_case(&object.name_singular);
    let type_name = format!("{}UpdateInput", singular);
    let projection = project_node(object, ProjectionOptions::default());
    let mut variables = Map::new();
    variables.insert(String::from("id"), Value::String(id));
    variables.insert(String::from("data"), data);
    let query = format!(
        "mutation UpdateOne{singular}($id: UUID!, $data: {type_name}!) {{ update{singular}(id: $id, data: $data) {{ {projection} }} }}"
    );
    Ok(GraphQLDocument::new(query, variables))
}

pub fn record_action_mutation(
    object: &ObjectMetadata,
    action: RecordAction,
    id: String,
) -> Result<GraphQLDocument, QueryBuildError> {
    validate_object_name(&object.name_singular)?;
    let singular = pascal_case(&object.name_singular);
    let projection = project_node(object, ProjectionOptions::default());
    let resolver = match action {
        RecordAction::Delete => "delete",
        RecordAction::Destroy => "destroy",
        RecordAction::Restore => "restore",
    };
    let mut variables = Map::new();
    variables.insert(String::from("id"), Value::String(id));
    let query = format!(
        "mutation {resolver_title}{singular}($id: UUID!) {{ {resolver}{singular}(id: $id) {{ {projection} }} }}",
        resolver_title = resolver
    );
    Ok(GraphQLDocument::new(query, variables))
}

#[derive(Debug, Clone, Copy)]
pub enum RecordAction {
    Delete,
    Destroy,
    Restore,
}

fn variable_type(name: &str, value: &Value, type_name: &str) -> String {
    match name {
        "filter" => format!("{type_name}FilterInput"),
        "orderBy" => format!("[{type_name}OrderByInput]"),
        "after" | "before" => String::from("String"),
        "offset" => String::from("Int"),
        "first" | "last" => String::from("Int"),
        _ if value.is_boolean() => String::from("Boolean"),
        _ if value.is_i64() || value.is_u64() => String::from("Int"),
        _ if value.is_f64() => String::from("Float"),
        _ => String::from("String"),
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchQueryArgs {
    pub search_input: String,
    pub limit: usize,
    pub after: Option<String>,
    pub included_object_name_singulars: Option<Vec<String>>,
    pub excluded_object_name_singulars: Option<Vec<String>>,
    pub filter: Option<Value>,
}

pub fn search_query(args: SearchQueryArgs) -> GraphQLDocument {
    let mut variables = Map::new();
    let mut definitions = vec![
        String::from("$searchInput: String!"),
        String::from("$limit: Int!"),
    ];
    variables.insert(
        String::from("searchInput"),
        Value::String(args.search_input),
    );
    variables.insert(
        String::from("limit"),
        Value::from(args.limit.min(100) as u64),
    );
    if let Some(after) = args.after.filter(|value| !value.is_empty()) {
        definitions.push(String::from("$after: String"));
        variables.insert(String::from("after"), Value::String(after));
    }
    if let Some(included) = args
        .included_object_name_singulars
        .filter(|values| !values.is_empty())
    {
        definitions.push(String::from("$includedObjectNameSingulars: [String!]"));
        variables.insert(
            String::from("includedObjectNameSingulars"),
            Value::Array(included.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(excluded) = args
        .excluded_object_name_singulars
        .filter(|values| !values.is_empty())
    {
        definitions.push(String::from("$excludedObjectNameSingulars: [String!]"));
        variables.insert(
            String::from("excludedObjectNameSingulars"),
            Value::Array(excluded.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(filter) = args.filter {
        definitions.push(String::from("$filter: ObjectRecordFilterInput"));
        variables.insert(String::from("filter"), filter);
    }
    let query = format!(
        "query Search({}) {{ search({}) {{ edges {{ node {{ recordId objectNameSingular objectLabelSingular label imageUrl tsRankCD tsRank }} cursor }} pageInfo {{ endCursor hasNextPage }} }} }}",
        definitions.join(", "),
        definitions
            .iter()
            .map(|definition| {
                let name = definition.split(':').next().expect("variable definition");
                let argument = name.strip_prefix('$').unwrap_or(name);
                format!("{argument}: {name}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
    GraphQLDocument::new(query, variables)
}

pub fn pascal_case(name: &str) -> String {
    let mut output = String::new();
    let mut uppercase = true;
    for character in name.chars() {
        if character.is_alphanumeric() {
            if uppercase {
                output.extend(character.to_uppercase());
                uppercase = false;
            } else {
                output.push(character);
            }
        } else {
            uppercase = true;
        }
    }
    output
}

fn validate_object_name(name: &str) -> Result<(), QueryBuildError> {
    let mut characters = name.chars();
    let valid_start = characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic());
    if !valid_start
        || !characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(QueryBuildError::InvalidObjectName(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn person() -> ObjectMetadata {
        serde_json::from_value(json!({
            "nameSingular": "person",
            "namePlural": "people",
            "fieldsList": [{"name": "id", "type": "UUID"}, {"name": "name", "type": "FULL_NAME"}]
        }))
        .unwrap()
    }

    #[test]
    fn find_many_query_matches_the_upstream_connection_shape() {
        let document = find_many_query(
            &person(),
            FindManyArgs {
                first: Some(2),
                filter: Some(json!({"name": {"lastName": {"eq": "Zhao"}}})),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(document.query.starts_with("query FindManyPeople("));
        assert!(document.query.contains("$first: Int"));
        assert!(document.query.contains("first: $first"));
        assert!(document
            .query
            .contains("edges { node { id name { firstName lastName } } cursor }"));
        assert!(document
            .query
            .contains("pageInfo { hasNextPage hasPreviousPage startCursor endCursor } totalCount"));
        assert_eq!(document.variables["first"], 2);
    }

    #[test]
    fn find_many_query_can_skip_the_total_count_scan() {
        let document = find_many_query(
            &person(),
            FindManyArgs {
                after: Some(String::from("cursor")),
                include_total_count: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!document.query.contains("totalCount"));
        assert!(document.query.contains("after: $after"));
    }

    #[test]
    fn create_one_mutation_uses_the_upstream_resolver_name_and_input_type() {
        let document =
            create_one_mutation(&person(), json!({"name": {"firstName": "Ada"}})).unwrap();
        assert_eq!(
            document.query,
            "mutation CreateOnePerson($data: PersonCreateInput!) { createPerson(data: $data) { id name { firstName lastName } } }"
        );
    }

    #[test]
    fn record_action_mutations_keep_the_upstream_singular_resolvers() {
        for (action, resolver) in [
            (RecordAction::Delete, "deletePerson"),
            (RecordAction::Destroy, "destroyPerson"),
            (RecordAction::Restore, "restorePerson"),
        ] {
            let document = record_action_mutation(&person(), action, "p1".into()).unwrap();
            assert!(document.query.contains(&format!("{resolver}(id: $id)")));
        }
    }

    #[test]
    fn search_query_uses_the_upstream_connection_shape() {
        let document = search_query(SearchQueryArgs {
            search_input: String::from("Ada"),
            limit: 20,
            after: Some(String::from("cursor")),
            included_object_name_singulars: Some(vec![String::from("person")]),
            excluded_object_name_singulars: Some(vec![String::from("task")]),
            filter: Some(json!({"id": {"eq": "8f3a0e5a-6f2d-4f4f-9a1e-2f7f0f6f4d51"}})),
        });
        assert!(document.query.starts_with(
            "query Search($searchInput: String!, $limit: Int!, $after: String, $includedObjectNameSingulars: [String!]"
        ));
        assert!(
            document
                .query
                .contains("search(searchInput: $searchInput, limit: $limit, after: $after"),
            "{}",
            document.query
        );
        assert!(document
            .query
            .contains("node { recordId objectNameSingular objectLabelSingular label imageUrl tsRankCD tsRank }"));
        assert!(document
            .query
            .contains("pageInfo { endCursor hasNextPage }"));
        assert_eq!(document.variables["limit"], 20);
        assert_eq!(document.variables["after"], "cursor");
    }

    #[test]
    fn invalid_object_names_are_rejected_before_query_generation() {
        let mut object = person();
        object.name_plural = "people); DROP TABLE".into();
        assert!(matches!(
            find_many_query(&object, FindManyArgs::default()),
            Err(QueryBuildError::InvalidObjectName(_))
        ));
    }
}
