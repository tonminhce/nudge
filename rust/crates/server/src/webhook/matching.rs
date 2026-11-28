pub const WILDCARD_EXCLUDED_EVENT_NAMES: [&str; 1] = ["workflowRun.updated"];

#[derive(Debug, Clone)]
pub struct WebhookSubscription {
    pub id: String,
    pub target_url: String,
    pub operations: Vec<String>,
    pub secret: String,
}

pub fn compute_webhook_operations_to_match(
    name_singular: &str,
    operation: Option<&str>,
) -> Vec<String> {
    let operation = operation.unwrap_or("undefined");
    let exact_operation = format!("{}.{}", name_singular, operation);

    if WILDCARD_EXCLUDED_EVENT_NAMES.contains(&exact_operation.as_str()) {
        return vec![exact_operation];
    }

    vec![
        exact_operation,
        format!("*.{}", operation),
        format!("{}.*", name_singular),
        "*.*".to_string(),
    ]
}

pub fn find_webhooks_matching_event_name<'a>(
    webhooks: impl IntoIterator<Item = &'a WebhookSubscription>,
    event_name: &str,
) -> Vec<&'a WebhookSubscription> {
    let mut parts = event_name.split('.');
    let name_singular = parts.next().unwrap_or("");
    let operation = parts.next();

    let operations_to_match = compute_webhook_operations_to_match(name_singular, operation);

    webhooks
        .into_iter()
        .filter(|webhook| {
            operations_to_match
                .iter()
                .any(|operation_to_match| webhook.operations.contains(operation_to_match))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn webhook(id: &str, operations: &[&str]) -> WebhookSubscription {
        WebhookSubscription {
            id: id.to_string(),
            target_url: "https://example.com/hook".to_string(),
            operations: operations.iter().map(|value| value.to_string()).collect(),
            secret: "secret".to_string(),
        }
    }

    #[test]
    fn operations_to_match_cover_exact_and_wildcards() {
        assert_eq!(
            compute_webhook_operations_to_match("person", Some("created")),
            vec![
                "person.created".to_string(),
                "*.created".to_string(),
                "person.*".to_string(),
                "*.*".to_string()
            ]
        );
    }

    #[test]
    fn workflow_run_updated_is_excluded_from_wildcards() {
        assert_eq!(
            compute_webhook_operations_to_match("workflowRun", Some("updated")),
            vec!["workflowRun.updated".to_string()]
        );
    }

    #[test]
    fn matching_selects_subscribed_webhooks_in_order() {
        let webhooks = vec![
            webhook("exact", &["person.created"]),
            webhook("object", &["person.*"]),
            webhook("operation", &["*.created"]),
            webhook("all", &["*.*"]),
            webhook("other", &["company.created"]),
            webhook("unrelated", &["person.deleted"]),
        ];

        let matched: Vec<&str> = find_webhooks_matching_event_name(&webhooks, "person.created")
            .into_iter()
            .map(|webhook| webhook.id.as_str())
            .collect();

        assert_eq!(matched, vec!["exact", "object", "operation", "all"]);
    }

    #[test]
    fn workflow_run_updated_only_matches_exact_subscription() {
        let webhooks = vec![
            webhook("exact", &["workflowRun.updated"]),
            webhook("all", &["*.*"]),
            webhook("operation", &["*.updated"]),
        ];

        let matched: Vec<&str> =
            find_webhooks_matching_event_name(&webhooks, "workflowRun.updated")
                .into_iter()
                .map(|webhook| webhook.id.as_str())
                .collect();

        assert_eq!(matched, vec!["exact"]);
    }

    #[test]
    fn event_name_without_operation_matches_undefined() {
        let webhooks = vec![webhook("undef", &["person.undefined"])];

        let matched = find_webhooks_matching_event_name(&webhooks, "person");

        assert_eq!(matched.len(), 1);
    }
}
