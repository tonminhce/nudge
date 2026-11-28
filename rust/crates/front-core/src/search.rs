use serde::Deserialize;
use serde_json::Value;

use crate::client::BrowserGraphQLClient;
use crate::client::ClientError;
use crate::client::Endpoint;
use crate::client::GraphQLClient;
use crate::client::RequestOptions;
use crate::query_builder::search_query;
use crate::query_builder::SearchQueryArgs;

pub const MAX_SEARCH_RESULTS: usize = 30;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchRecord {
    pub record_id: String,
    pub object_name_singular: String,
    pub object_label_singular: String,
    pub label: String,
    pub image_url: Option<String>,
    #[serde(rename = "tsRankCD")]
    pub ts_rank_cd: f64,
    pub ts_rank: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SearchEdge {
    pub node: SearchRecord,
    pub cursor: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchPageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchConnection {
    pub edges: Vec<SearchEdge>,
    pub page_info: SearchPageInfo,
}

pub async fn load_search_records(
    base_url: &str,
    token: &str,
    args: SearchQueryArgs,
) -> Result<SearchConnection, ClientError> {
    let document = search_query(args);
    let data = BrowserGraphQLClient::new()
        .execute_data(
            base_url,
            &document.query,
            RequestOptions::new(Endpoint::Core)
                .operation_name("Search")
                .variables(Value::Object(document.variables))
                .bearer_token(token),
        )
        .await?;
    serde_json::from_value(data["search"].clone()).map_err(ClientError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn search_connection_deserializes_the_upstream_shape() {
        let connection: SearchConnection = serde_json::from_value(json!({
            "edges": [{
                "node": {
                    "recordId": "person-1",
                    "objectNameSingular": "person",
                    "objectLabelSingular": "Person",
                    "label": "Ada Lovelace",
                    "imageUrl": null,
                    "tsRankCD": 0.1,
                    "tsRank": 0.06079271
                },
                "cursor": "cursor-1"
            }],
            "pageInfo": {"endCursor": "cursor-1", "hasNextPage": true}
        }))
        .expect("search connection");

        assert_eq!(connection.edges[0].node.label, "Ada Lovelace");
        assert_eq!(connection.edges[0].node.image_url, None);
        assert_eq!(connection.page_info.end_cursor.as_deref(), Some("cursor-1"));
        assert!(connection.page_info.has_next_page);
    }

    #[test]
    fn maximum_search_results_match_the_upstream_command_menu() {
        assert_eq!(MAX_SEARCH_RESULTS, 30);
    }
}
