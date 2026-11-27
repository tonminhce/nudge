use front_core::client::Endpoint;
use front_core::client::GraphQLClient;
use front_core::client::RequestOptions;
use front_core::client::ReqwestGraphQLClient;
use front_core::client_config::fetch_client_config;
use front_core::metadata_store::FindMinimalMetadata;
use front_core::metadata_store::MetadataStore;
use front_core::optimistic::OptimisticEngine;
use front_core::projection::ObjectMetadata;
use front_core::query_builder::find_many_query;
use front_core::query_builder::FindManyArgs;
use front_core::query_cache::QueryCache;
use front_core::query_cache::QueryKey;
use front_core::query_cache::RecordIndex;
use serde_json::json;

#[tokio::test]
#[ignore = "requires PARITY_TOKEN and a live Twenty reference"]
async fn front_core_round_trips_the_live_reference_metadata_and_people_query() {
    let base_url =
        std::env::var("PARITY_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());
    let token = std::env::var("PARITY_TOKEN").expect("PARITY_TOKEN");
    let client = ReqwestGraphQLClient::new();

    let config = fetch_client_config(&base_url).await.unwrap();
    assert!(config.app_version.starts_with("v2.61"));

    let minimal_query = r#"query { minimalMetadata { objectMetadataItems { id nameSingular namePlural labelSingular labelPlural icon isActive isSystem isRemote } views { id type key objectMetadataId } collectionHashes { collectionName hash } } }"#;
    let minimal_response = client
        .execute_data(
            &base_url,
            minimal_query,
            RequestOptions::new(Endpoint::Metadata).bearer_token(token.clone()),
        )
        .await
        .unwrap();
    let minimal = FindMinimalMetadata::from_response(&minimal_response).unwrap();
    assert!(minimal.object_metadata_items.len() >= 37);
    assert!(minimal.views.len() >= 50);
    let mut store = MetadataStore::new();
    store.hydrate_minimal(minimal);
    assert!(store.gate_is_open());

    let objects_query = r#"query { objects(paging: { first: 100 }, filter: {}) { edges { node { nameSingular namePlural fieldsList { id name type settings relation { type targetObjectMetadata { nameSingular namePlural fieldsList { name type } } } morphId } } } } }"#;
    let objects_response = client
        .execute_data(
            &base_url,
            objects_query,
            RequestOptions::new(Endpoint::Metadata).bearer_token(token.clone()),
        )
        .await
        .unwrap();
    let objects = ObjectMetadata::from_metadata_response(&objects_response);
    let person = objects
        .iter()
        .find(|object| object.name_singular == "person")
        .expect("person metadata");
    let document = find_many_query(
        person,
        FindManyArgs {
            first: Some(2),
            ..Default::default()
        },
    )
    .unwrap();
    let people_response = client
        .execute_data(
            &base_url,
            &document.query,
            RequestOptions::new(Endpoint::Core)
                .operation_name("FindManyPeople")
                .variables(serde_json::Value::Object(document.variables.clone()))
                .bearer_token(token),
        )
        .await
        .unwrap();
    let edges = people_response["people"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(people_response["people"]["totalCount"].as_i64().unwrap() >= 2);
    assert!(edges[0]["node"].get("companyId").is_some());
    assert!(edges[0]["node"].get("company").is_none());

    let record_id = edges[0]["node"]["id"].as_str().unwrap().to_string();
    let key = QueryKey::new("FindManyPeople", Endpoint::Core, Some(&json!({"first": 2})));
    let mut cache = QueryCache::new();
    let mut records = RecordIndex::new();
    records.bind_query(key.clone(), "person", &people_response);
    cache.insert(key, people_response);
    let mut engine = OptimisticEngine::new(cache, records);
    let snapshot = engine.edit_field("person", &record_id, "jobTitle", json!("Front-core parity"));
    assert_eq!(
        engine.records.get("person", &record_id).unwrap()["jobTitle"],
        "Front-core parity"
    );
    engine.rollback(snapshot);
    assert_ne!(
        engine.records.get("person", &record_id).unwrap()["jobTitle"],
        "Front-core parity"
    );
}
