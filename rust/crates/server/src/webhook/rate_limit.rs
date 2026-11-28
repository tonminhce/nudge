use redis::aio::MultiplexedConnection;
use redis::Script;
use serde_json::{json, Map, Value};

pub const TOKEN_BUCKET_THROTTLE_KEY_PREFIX: &str = "token-bucket";
pub const WEBHOOK_CALL_RATE_LIMITING_TTL_IN_MS: u64 = 60_000;
pub const WEBHOOK_CALL_RATE_LIMITING_LIMIT: u64 = 60;
pub const TOKEN_BUCKETS_ALLOW_PARTIAL_ARG: &str = "1";
pub const TOKEN_BUCKETS_DENY_PARTIAL_ARG: &str = "0";

pub const TRY_CONSUME_TOKEN_BUCKETS_SCRIPT: &str = r###"
local requestedCost = tonumber(ARGV[1])
local buckets = cjson.decode(ARGV[2])
local allowPartial = ARGV[3] == '1'

if #buckets ~= #KEYS then
  return redis.error_reply(
    'try-consume-token-buckets: ' .. #KEYS .. ' keys but ' .. #buckets .. ' bucket configs'
  )
end

local now = redis.call('TIME')
local nowMs = tonumber(now[1]) * 1000 + math.floor(tonumber(now[2]) / 1000)
local available = {}
local exhaustedIndex = 0
local retryAfterMs = 0
local admittedCost = requestedCost

for i = 1, #KEYS do
  local bucket = buckets[i]
  local state = redis.call('HMGET', KEYS[i], 't', 'ts')
  local tokens = tonumber(state[1])
  local lastRefillAt = tonumber(state[2])

  if tokens == nil or lastRefillAt == nil then
    tokens = bucket.burst
    lastRefillAt = nowMs
  end

  local elapsed = nowMs - lastRefillAt
  if elapsed < 0 then elapsed = 0 end

  local refilled = tokens + (elapsed * bucket.refill / bucket.windowMs)
  if refilled > bucket.burst then refilled = bucket.burst end
  available[i] = refilled

  local bucketAdmittedCost = math.floor(refilled)
  if bucketAdmittedCost > requestedCost then bucketAdmittedCost = requestedCost end
  if bucketAdmittedCost < admittedCost then admittedCost = bucketAdmittedCost end

  if bucketAdmittedCost < requestedCost then
    local bucketRetryAfterMs = math.ceil((requestedCost - refilled) * bucket.windowMs / bucket.refill)

    -- Buckets arrive narrowest first, so the first exhausted one names the
    -- scope, while the caller has to wait for the slowest of them to refill.
    if exhaustedIndex == 0 then
      exhaustedIndex = i
    end

    if bucketRetryAfterMs > retryAfterMs then
      retryAfterMs = bucketRetryAfterMs
    end
  end
end

if not allowPartial and exhaustedIndex > 0 then
  admittedCost = 0
end

if admittedCost <= 0 then
  return { 0, exhaustedIndex, retryAfterMs }
end

for i = 1, #KEYS do
  local bucket = buckets[i]
  redis.call('HSET', KEYS[i], 't', tostring(available[i] - admittedCost), 'ts', tostring(nowMs))
  -- Never expire before an empty bucket would have refilled, or the key
  -- vanishing hands back a full burst the elapsed time had not yet earned.
  redis.call('PEXPIRE', KEYS[i], math.ceil(bucket.burst / bucket.refill * bucket.windowMs))
end

return { admittedCost, exhaustedIndex, retryAfterMs }
"###;

pub fn build_speed_bucket_key(
    workspace_id: &str,
    resource_type: &str,
    operation_type: &str,
    spender_type: &str,
    spender_id: Option<&str>,
    window_seconds: u64,
) -> String {
    format!(
        "{{{}}}:speed:{}:{}:{}:{}:{}",
        workspace_id,
        resource_type,
        operation_type,
        spender_type,
        spender_id.filter(|value| !value.is_empty()).unwrap_or("-"),
        window_seconds
    )
}

pub fn webhook_speed_bucket_key(workspace_id: &str) -> String {
    build_speed_bucket_key(
        workspace_id,
        "WEBHOOK",
        "WEBHOOK_CALL",
        "workspace",
        None,
        WEBHOOK_CALL_RATE_LIMITING_TTL_IN_MS.div_ceil(1_000),
    )
}

pub fn throttled_cache_key(key: &str) -> String {
    format!("{}:{}", TOKEN_BUCKET_THROTTLE_KEY_PREFIX, key)
}

pub fn bucket_config_json(window_ms: u64, limit: u64) -> String {
    json!([{
        "burst": limit,
        "refill": limit,
        "windowMs": window_ms,
    }])
    .to_string()
}

pub struct SpeedBucketOutcome {
    pub admitted_count: u64,
    pub exhausted_bucket_position: u64,
    pub retry_after_ms: u64,
}

impl SpeedBucketOutcome {
    pub fn admitted(&self) -> bool {
        self.exhausted_bucket_position == 0
    }
}

pub async fn try_consume_token_buckets(
    connection: &mut MultiplexedConnection,
    keys: &[String],
    tokens_to_consume: u64,
    buckets_json: &str,
    allow_partial: bool,
) -> Result<SpeedBucketOutcome, redis::RedisError> {
    let script = Script::new(TRY_CONSUME_TOKEN_BUCKETS_SCRIPT);
    let mut invocation = script.arg(tokens_to_consume);
    invocation.arg(buckets_json);
    invocation.arg(if allow_partial {
        TOKEN_BUCKETS_ALLOW_PARTIAL_ARG
    } else {
        TOKEN_BUCKETS_DENY_PARTIAL_ARG
    });

    for key in keys {
        invocation.key(key);
    }

    let results: Vec<u64> = invocation.invoke_async(connection).await?;

    let [admitted_count, exhausted_bucket_position, retry_after_ms] =
        parse_outcome(&results).ok_or_else(|| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "unexpected throttler script outcome",
            ))
        })?;

    Ok(SpeedBucketOutcome {
        admitted_count,
        exhausted_bucket_position,
        retry_after_ms,
    })
}

fn parse_outcome(results: &[u64]) -> Option<[u64; 3]> {
    if results.len() != 3 {
        return None;
    }

    Some([results[0], results[1], results[2]])
}

pub async fn admit_webhook_event_count(
    connection: &mut MultiplexedConnection,
    workspace_id: &str,
    requested: u64,
) -> Result<u64, redis::RedisError> {
    if requested == 0 {
        return Ok(0);
    }

    let outcome = try_consume_token_buckets(
        connection,
        &[throttled_cache_key(&webhook_speed_bucket_key(workspace_id))],
        requested,
        &bucket_config_json(
            WEBHOOK_CALL_RATE_LIMITING_TTL_IN_MS,
            WEBHOOK_CALL_RATE_LIMITING_LIMIT,
        ),
        true,
    )
    .await?;

    Ok(outcome.admitted_count)
}

pub fn slice_admitted_events<T>(events: Vec<T>, admitted_count: u64) -> Vec<T> {
    events.into_iter().take(admitted_count as usize).collect()
}

pub fn build_webhook_response_event_properties(
    url: &str,
    webhook_id: &str,
    event_name: &str,
    success: bool,
    status: Option<u16>,
    error: Option<&str>,
) -> Value {
    let mut properties = Map::new();

    if success {
        if let Some(status) = status {
            properties.insert("status".to_string(), json!(status));
        }

        properties.insert("success".to_string(), json!(success));
    } else {
        properties.insert("success".to_string(), json!(success));
    }

    properties.insert("url".to_string(), json!(url));
    properties.insert("webhookId".to_string(), json!(webhook_id));
    properties.insert("eventName".to_string(), json!(event_name));

    if !success {
        if let Some(status) = status {
            properties.insert("status".to_string(), json!(status));
        }

        if let Some(error) = error {
            properties.insert("error".to_string(), json!(error));
        }
    }

    Value::Object(properties)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_bucket_key_uses_absent_spender_marker() {
        assert_eq!(
            webhook_speed_bucket_key("7c9e6679-7425-40de-944b-e07fc1f90ae7"),
            "{7c9e6679-7425-40de-944b-e07fc1f90ae7}:speed:WEBHOOK:WEBHOOK_CALL:workspace:-:60"
        );
    }

    #[test]
    fn cache_key_carries_token_bucket_prefix() {
        assert_eq!(
            throttled_cache_key("bucket"),
            "token-bucket:bucket".to_string()
        );
    }

    #[test]
    fn bucket_config_matches_upstream_shape() {
        assert_eq!(
            bucket_config_json(60_000, 60),
            "[{\"burst\":60,\"refill\":60,\"windowMs\":60000}]"
        );
    }

    #[test]
    fn slice_admitted_keeps_prefix_of_events() {
        let events: Vec<u32> = (0..25).collect();
        let admitted = slice_admitted_events(events, 3);

        assert_eq!(admitted, vec![0, 1, 2]);
    }

    #[test]
    fn parse_outcome_requires_the_three_upstream_positions() {
        assert_eq!(parse_outcome(&[3, 0, 0]), Some([3u64, 0u64, 0u64]));
        assert_eq!(parse_outcome(&[3, 0]), None);
        assert_eq!(parse_outcome(&[]), None);
    }

    #[test]
    fn outcome_is_admitted_only_without_an_exhausted_bucket() {
        assert!(SpeedBucketOutcome {
            admitted_count: 2,
            exhausted_bucket_position: 0,
            retry_after_ms: 0,
        }
        .admitted());
        assert!(!SpeedBucketOutcome {
            admitted_count: 1,
            exhausted_bucket_position: 1,
            retry_after_ms: 500,
        }
        .admitted());
    }

    #[test]
    fn partial_arg_matches_the_upstream_script_arguments() {
        assert_eq!(TOKEN_BUCKETS_ALLOW_PARTIAL_ARG, "1");
        assert_eq!(TOKEN_BUCKETS_DENY_PARTIAL_ARG, "0");
        assert!(TRY_CONSUME_TOKEN_BUCKETS_SCRIPT.contains("local allowPartial = ARGV[3] == '1'"));
        assert!(TRY_CONSUME_TOKEN_BUCKETS_SCRIPT
            .contains("return { admittedCost, exhaustedIndex, retryAfterMs }"));
    }

    #[test]
    fn webhook_response_properties_follow_upstream_shape() {
        let properties = build_webhook_response_event_properties(
            "https://example.com/hook",
            "webhook-1",
            "person.created",
            false,
            Some(500),
            None,
        );

        assert_eq!(
            serde_json::to_string(&properties).unwrap(),
            "{\"success\":false,\"url\":\"https://example.com/hook\",\"webhookId\":\"webhook-1\",\"eventName\":\"person.created\",\"status\":500}"
        );
    }

    #[test]
    fn webhook_response_success_variant_leads_with_status() {
        let properties = build_webhook_response_event_properties(
            "https://example.com/hook",
            "webhook-1",
            "person.created",
            true,
            Some(204),
            None,
        );

        assert_eq!(
            serde_json::to_string(&properties).unwrap(),
            "{\"status\":204,\"success\":true,\"url\":\"https://example.com/hook\",\"webhookId\":\"webhook-1\",\"eventName\":\"person.created\"}"
        );
    }

    #[test]
    fn webhook_response_error_variant_carries_message_and_no_status() {
        let properties = build_webhook_response_event_properties(
            "https://example.com/hook",
            "webhook-1",
            "person.created",
            false,
            None,
            Some("timeout"),
        );

        assert_eq!(
            serde_json::to_string(&properties).unwrap(),
            "{\"success\":false,\"url\":\"https://example.com/hook\",\"webhookId\":\"webhook-1\",\"eventName\":\"person.created\",\"error\":\"timeout\"}"
        );
    }
}
