use front_core::client::BrowserGraphQLClient;
use front_core::client::Endpoint;
use front_core::client::GraphQLClient;
use front_core::client::RequestOptions;
use front_core::projection::ObjectMetadata;
use front_core::query_builder::find_many_query;
use front_core::query_builder::pascal_case;
use front_core::query_builder::FindManyArgs;
use serde_json::Value;

const OBJECTS_QUERY: &str = r#"query { objects(paging: { first: 100 }, filter: {}) { edges { node { nameSingular namePlural fieldsList { id name type settings relation { type targetObjectMetadata { nameSingular namePlural fieldsList { name type } } } morphId } } } } }"#;

pub async fn load_object_metadata(token: &str) -> Result<Vec<ObjectMetadata>, String> {
    let response = BrowserGraphQLClient::default()
        .execute_data(
            &crate::app::server_url(),
            OBJECTS_QUERY,
            RequestOptions::new(Endpoint::Metadata).bearer_token(token),
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(ObjectMetadata::from_metadata_response(&response))
}

pub async fn load_records(
    token: &str,
    object: &ObjectMetadata,
    after: Option<String>,
    filter: Option<Value>,
    order_by: Option<Value>,
    depth: usize,
    include_total_count: bool,
) -> Result<Value, String> {
    let document = find_many_query(
        object,
        FindManyArgs {
            first: Some(60),
            after,
            filter,
            order_by,
            depth,
            include_total_count: Some(include_total_count),
            ..Default::default()
        },
    )
    .map_err(|error| error.to_string())?;
    BrowserGraphQLClient::default()
        .execute_data(
            &crate::app::server_url(),
            &document.query,
            RequestOptions::new(Endpoint::Core)
                .variables(Value::Object(document.variables))
                .bearer_token(token),
        )
        .await
        .map_err(|error| error.to_string())
}

pub fn records_from_response(response: &Value, plural: &str) -> Vec<Value> {
    response[plural]["edges"]
        .as_array()
        .map(|edges| {
            edges
                .iter()
                .filter_map(|edge| edge.get("node").cloned())
                .collect()
        })
        .unwrap_or_default()
}

pub async fn create_record(
    token: &str,
    object: &ObjectMetadata,
    data: Value,
) -> Result<Value, String> {
    let document = front_core::query_builder::create_one_mutation(object, data)
        .map_err(|error| error.to_string())?;
    let response = BrowserGraphQLClient::default()
        .execute_data(
            &crate::app::server_url(),
            &document.query,
            RequestOptions::new(Endpoint::Core)
                .variables(Value::Object(document.variables))
                .bearer_token(token),
        )
        .await
        .map_err(|error| error.to_string())?;
    response
        .get(format!("create{}", pascal_case(&object.name_singular)))
        .cloned()
        .ok_or_else(|| "The create mutation returned no record".to_string())
}

pub async fn update_record(
    token: &str,
    object: &ObjectMetadata,
    id: &str,
    data: Value,
) -> Result<Value, String> {
    let document = front_core::query_builder::update_one_mutation(object, id.to_string(), data)
        .map_err(|error| error.to_string())?;
    let response = BrowserGraphQLClient::default()
        .execute_data(
            &crate::app::server_url(),
            &document.query,
            RequestOptions::new(Endpoint::Core)
                .variables(Value::Object(document.variables))
                .bearer_token(token),
        )
        .await
        .map_err(|error| error.to_string())?;
    response
        .get(format!("update{}", pascal_case(&object.name_singular)))
        .cloned()
        .ok_or_else(|| "The update mutation returned no record".to_string())
}

pub fn end_cursor(response: &Value, plural: &str) -> Option<String> {
    response[plural]["pageInfo"]["endCursor"]
        .as_str()
        .map(str::to_string)
}
