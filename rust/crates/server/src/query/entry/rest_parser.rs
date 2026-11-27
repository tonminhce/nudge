use serde_json::Map;
use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;

use crate::query::entry::rest::RestError;

pub const FILTER_COMPARATORS: [&str; 13] = [
    "eq",
    "neq",
    "in",
    "containsAny",
    "is",
    "gt",
    "gte",
    "lt",
    "lte",
    "startsWith",
    "endsWith",
    "like",
    "ilike",
];

pub fn parse_filter(raw: Option<&str>) -> Result<Option<Value>, RestError> {
    let Some(raw) = raw.filter(|raw| !raw.trim().is_empty()) else {
        return Ok(None);
    };

    let opens = raw.matches('(').count();
    let closes = raw.matches(')').count();

    if opens > closes {
        let hint = if opens - closes == 1 {
            String::from("1 close bracket is")
        } else {
            format!("{} close brackets are", opens - closes)
        };
        return Err(RestError::invalid_filter(format!(
            "'filter' invalid. {hint} missing in the query"
        )));
    }

    if closes > opens {
        let hint = if closes - opens == 1 {
            String::from("1 open bracket is")
        } else {
            format!("{} open brackets are", closes - opens)
        };
        return Err(RestError::invalid_filter(format!(
            "'filter' invalid. {hint} missing in the query"
        )));
    }

    let mut parser = FilterParser {
        input: raw.trim(),
        position: 0,
    };
    let value = parser.parse_condition()?;
    parser.skip_whitespace();

    if parser.position != parser.input.len() {
        return Err(RestError::invalid_filter(format!(
            "'filter' invalid for '{}'. eg: price[gte]:10",
            parser.input[parser.position..].trim()
        )));
    }

    Ok(Some(value))
}

pub fn parse_order_by(raw: Option<&str>) -> Result<Option<Value>, RestError> {
    let Some(raw) = raw.filter(|raw| !raw.trim().is_empty()) else {
        return Ok(None);
    };

    let mut orders = Vec::new();

    for part in raw.split(',') {
        let part = part.trim();
        let Some(open) = part.find('[') else {
            orders.push(nested_order(part, "AscNullsFirst")?);
            continue;
        };

        if !part.ends_with(']') {
            return Err(RestError::invalid_order_by(
                "The order_by query param is invalid",
            ));
        }

        let field = &part[..open];
        let direction = &part[open + 1..part.len() - 1];

        if !matches!(
            direction,
            "AscNullsFirst" | "AscNullsLast" | "DescNullsFirst" | "DescNullsLast"
        ) {
            return Err(RestError::invalid_order_by(
                "The order_by query param is invalid",
            ));
        }

        orders.push(nested_order(field, direction)?);
    }

    Ok(Some(Value::Array(orders)))
}

fn nested_order(path: &str, direction: &str) -> Result<Value, RestError> {
    if path.is_empty() {
        return Err(RestError::invalid_order_by(
            "The order_by query param is invalid",
        ));
    }

    let mut object = Map::new();
    object.insert(String::from(path), Value::String(direction.to_string()));

    Ok(nested_filter_object(object))
}

pub fn parse_fields(raw: Option<&str>) -> Result<Option<Vec<String>>, RestError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let fields = raw
        .split(',')
        .map(|field| field.trim().to_string())
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();

    if fields.is_empty() {
        return Err(RestError::invalid_fields(
            "'fields' parameter is empty. Expected a comma-separated list of field names, e.g. fields=id,name",
        ));
    }

    Ok(Some(fields))
}

pub fn parse_bool(raw: Option<&str>) -> Result<Option<bool>, RestError> {
    match raw {
        None => Ok(None),
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(_) => Err(RestError::bad_request(
            "Boolean query param must be true or false",
        )),
    }
}

pub fn parse_limit(raw: Option<&str>) -> Result<Option<usize>, RestError> {
    match raw {
        None => Ok(None),
        Some(raw) => raw.parse::<usize>().map(Some).map_err(|_| {
            RestError::invalid_limit(format!("limit '{raw}' is invalid. Should be an integer"))
        }),
    }
}

pub fn parse_depth(raw: Option<&str>) -> Result<usize, RestError> {
    match raw {
        None | Some("0") => Ok(0),
        Some("1") => Ok(1),
        Some(raw) => Err(RestError::invalid_depth(format!(
            "'depth={raw}' parameter invalid. Allowed values are 0, 1"
        ))),
    }
}

pub fn parse_group_by(raw: Option<&str>) -> Result<Vec<(String, Option<String>)>, RestError> {
    let Some(raw) = raw.filter(|raw| !raw.trim().is_empty()) else {
        return Ok(Vec::new());
    };

    let parsed: Value = serde_json::from_str(raw)
        .map_err(|_| RestError::invalid_group_by("The group_by query param is invalid"))?;
    let Value::Array(items) = parsed else {
        return Err(RestError::invalid_group_by(
            "The group_by query param is invalid",
        ));
    };

    let mut definitions = Vec::new();

    for item in items {
        let Some(object) = item.as_object() else {
            return Err(RestError::invalid_group_by(
                "The group_by query param is invalid",
            ));
        };

        let Some((field, value)) = object.into_iter().next() else {
            return Err(RestError::invalid_group_by(
                "The group_by query param is invalid",
            ));
        };

        let granularity = match value {
            Value::Bool(true) => Some(String::from("NONE")),
            Value::Object(settings) => settings
                .get("granularity")
                .and_then(Value::as_str)
                .map(str::to_string),
            _ => None,
        };

        definitions.push((field.clone(), granularity));
    }

    Ok(definitions)
}

pub fn parse_aggregates(raw: Option<&str>) -> Result<Vec<Value>, RestError> {
    let Some(raw) = raw.filter(|raw| !raw.trim().is_empty()) else {
        return Ok(Vec::new());
    };

    let parsed: Value = serde_json::from_str(raw)
        .map_err(|_| RestError::invalid_aggregates("The aggregate query param is invalid"))?;
    let Value::Array(items) = parsed else {
        return Err(RestError::invalid_aggregates(
            "The aggregate query param is invalid",
        ));
    };

    Ok(items)
}

pub fn pick_fields(record: &mut Map<String, Value>, fields: Option<&[String]>) {
    let Some(fields) = fields else {
        return;
    };

    let id = record.get("id").cloned();
    let kept = record
        .iter()
        .filter(|(key, _)| key.as_str() != "id" && fields.contains(*key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<_>>();

    record.clear();

    if let Some(id) = id {
        record.insert(String::from("id"), id);
    }

    for (key, value) in kept {
        record.insert(key, value);
    }
}

struct FilterParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> FilterParser<'a> {
    fn skip_whitespace(&mut self) {
        self.position += self.input[self.position..].len().min(
            self.input[self.position..]
                .find(|character: char| !character.is_whitespace())
                .unwrap_or(self.input.len() - self.position),
        );
    }

    fn starts_with_conjunction(&self, keyword: &str) -> bool {
        self.input[self.position..].starts_with(keyword)
            && self
                .input
                .as_bytes()
                .get(self.position + keyword.len())
                .is_some_and(|character| *character == b'(')
    }

    fn parse_condition(&mut self) -> Result<Value, RestError> {
        self.skip_whitespace();

        for keyword in ["and", "or", "not"] {
            if self.starts_with_conjunction(keyword) {
                return self.parse_group(keyword);
            }
        }

        self.parse_leaf()
    }

    fn parse_group(&mut self, keyword: &str) -> Result<Value, RestError> {
        self.position += keyword.len() + 1;
        let mut conditions = Vec::new();

        loop {
            conditions.push(self.parse_condition()?);
            self.skip_whitespace();

            match self.input.as_bytes().get(self.position) {
                Some(b')') => {
                    self.position += 1;
                    break;
                }
                Some(b',') => self.position += 1,
                _ => {
                    return Err(RestError::bad_request(
                        "Error while matching filter query. Conjunction not found",
                    ))
                }
            }
        }

        if keyword == "not" && conditions.len() != 1 {
            return Err(RestError::invalid_filter(
                "'filter' invalid. 'not' conjunction should contain only 1 condition. eg: not(field[eq]:1)",
            ));
        }

        let mut object = Map::new();
        object.insert(keyword.to_string(), Value::Array(conditions));

        Ok(Value::Object(object))
    }

    fn parse_leaf(&mut self) -> Result<Value, RestError> {
        let start = self.position;
        let base = self.input[start..]
            .split(['(', ')', ','])
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        let Some(bracket) = self.input[start..].find('[') else {
            return Err(RestError::invalid_filter(format!(
                "'filter' invalid for '{base}'. eg: price[gte]:10"
            )));
        };

        let path = self.input[start..start + bracket].trim().to_string();
        self.position = start + bracket + 1;
        let Some(close) = self.input[self.position..].find(']') else {
            return Err(RestError::invalid_filter(format!(
                "'filter' invalid for '{path}'. eg: price[gte]:10"
            )));
        };

        let operator = self.input[self.position..self.position + close].to_string();
        self.position += close + 1;

        if self.input.as_bytes().get(self.position) != Some(&b':') {
            return Err(RestError::invalid_filter(format!(
                "'filter' invalid for '{path}'. eg: price[gte]:10"
            )));
        }

        self.position += 1;
        let value = self.parse_value()?;

        if !FILTER_COMPARATORS.contains(&operator.as_str()) {
            return Err(RestError::invalid_filter(format!(
                "'filter' invalid for '{path}', comparator {operator} not in {}",
                FILTER_COMPARATORS.join(", ")
            )));
        }

        let mut object = Map::new();
        let mut operator_object = Map::new();
        operator_object.insert(operator, value);
        object.insert(path, Value::Object(operator_object));

        Ok(nested_filter_object(object))
    }

    fn parse_value(&mut self) -> Result<Value, RestError> {
        self.skip_whitespace();

        if self.input.as_bytes().get(self.position) == Some(&b'[') {
            let close = self.input[self.position..]
                .find(']')
                .ok_or_else(|| RestError::invalid_filter("The filter query param is invalid"))?;
            let raw = &self.input[self.position..self.position + close + 1];
            self.position += close + 1;

            return serde_json::from_str(raw)
                .map_err(|_| RestError::invalid_filter("The filter query param is invalid"));
        }

        let start = self.position;

        while let Some(character) = self.input.as_bytes().get(self.position) {
            if *character == b',' || *character == b')' {
                break;
            }

            self.position += 1;
        }

        let raw = self.input[start..self.position].trim();
        let raw = raw
            .strip_prefix('"')
            .and_then(|raw| raw.strip_suffix('"'))
            .or_else(|| {
                raw.strip_prefix('\'')
                    .and_then(|raw| raw.strip_suffix('\''))
            })
            .unwrap_or(raw);

        if raw == "null" {
            return Ok(Value::Null);
        }

        if raw == "true" || raw == "false" {
            return Ok(Value::Bool(raw == "true"));
        }

        if let Ok(number) = raw.parse::<i64>() {
            return Ok(Value::Number(number.into()));
        }

        if let Ok(number) = raw.parse::<f64>() {
            return Ok(Value::Number(
                serde_json::Number::from_f64(number).unwrap_or_else(|| 0.into()),
            ));
        }

        Ok(Value::String(raw.to_string()))
    }
}

fn nested_filter_object(object: Map<String, Value>) -> Value {
    let mut nested = Map::new();

    for (path, value) in object {
        let segments = path.split('.').collect::<Vec<_>>();
        let mut leaf = value;
        let mut leaf_key = segments[segments.len() - 1].to_string();

        for segment in segments[..segments.len() - 1].iter().rev() {
            let mut container = Map::new();
            container.insert(leaf_key, leaf);
            leaf = Value::Object(container);
            leaf_key = (*segment).to_string();
        }

        nested.insert(leaf_key, leaf);
    }

    Value::Object(nested)
}

pub fn coerce_filter_values(filter: &mut Value) {
    match filter {
        Value::Object(object) => {
            for (_, value) in object.iter_mut() {
                coerce_filter_values(value);
            }
        }
        Value::Array(items) => {
            for item in items {
                coerce_filter_values(item);
            }
        }
        Value::String(raw) => {
            if let Ok(number) = raw.parse::<i64>() {
                *filter = Value::Number(number.into());
            } else if raw == "true" || raw == "false" {
                *filter = Value::Bool(raw == "true");
            } else if raw == "NULL" {
                *filter = Value::String(String::from("NULL"));
            }
        }
        _ => {}
    }
}

pub fn field_kind_marker(field_type: FieldMetadataType) -> &'static str {
    match field_type {
        FieldMetadataType::Number | FieldMetadataType::Numeric => "number",
        _ => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn filter_dsl_matches_the_upstream_leaf_and_logical_grammar() {
        let filter = parse_filter(Some("name[ilike]:%a%")).expect("leaf filter");

        assert_eq!(filter, Some(json!({"name": {"ilike": "%a%"}})));

        let filter = parse_filter(Some("and(price[gte]:10,name[eq]:Acme)")).expect("and filter");
        assert_eq!(
            filter,
            Some(json!({
                "and": [
                    {"price": {"gte": 10}},
                    {"name": {"eq": "Acme"}},
                ]
            }))
        );

        let filter = parse_filter(Some("not(name[eq]:Acme)")).expect("not filter");
        assert_eq!(filter, Some(json!({"not": [{"name": {"eq": "Acme"}}]})));

        let filter = parse_filter(Some("id[in]:[\"a\",\"b\"]")).expect("in filter");
        assert_eq!(filter, Some(json!({"id": {"in": ["a", "b"]}})));

        let filter = parse_filter(Some("company.name[eq]:Acme")).expect("nested filter");
        assert_eq!(filter, Some(json!({"company": {"name": {"eq": "Acme"}}})));

        let filter = parse_filter(Some("name[regex]:Acme")).expect_err("unsupported operator");
        assert_eq!(filter.code, "INVALID_FILTER_QUERY_PARAM");
    }

    #[test]
    fn order_by_and_fields_parse_to_runner_arguments() {
        let order = parse_order_by(Some("name[AscNullsLast],id")).expect("order by");
        assert_eq!(
            order,
            Some(json!([
                {"name": "AscNullsLast"},
                {"id": "AscNullsFirst"},
            ]))
        );

        let order = parse_order_by(Some("company.name[DescNullsFirst]")).expect("nested order");
        assert_eq!(
            order,
            Some(json!([{"company": {"name": "DescNullsFirst"}}]))
        );

        assert_eq!(
            parse_order_by(Some("name[Wrong]")).unwrap_err().code,
            "INVALID_ORDER_BY_QUERY_PARAM"
        );
        assert_eq!(
            parse_fields(Some("id,name.firstName")).expect("fields"),
            Some(vec![String::from("id"), String::from("name.firstName")])
        );
        assert_eq!(
            parse_fields(Some(" ")).unwrap_err().code,
            "INVALID_FIELDS_QUERY_PARAM"
        );
    }

    #[test]
    fn limit_cursor_depth_and_booleans_follow_the_upstream_contracts() {
        assert_eq!(parse_limit(Some("2")).expect("limit"), Some(2));
        assert_eq!(
            parse_limit(Some("bad")).unwrap_err().code,
            "INVALID_LIMIT_QUERY_PARAM"
        );
        assert_eq!(parse_depth(Some("1")).expect("depth"), 1);
        assert_eq!(
            parse_depth(Some("2")).unwrap_err().code,
            "INVALID_DEPTH_QUERY_PARAM"
        );
        assert_eq!(parse_bool(Some("true")).expect("bool"), Some(true));
        assert_eq!(parse_bool(Some("yes")).unwrap_err().code, "BAD_REQUEST");
    }

    #[test]
    fn record_fields_are_trimmed_while_id_is_always_kept() {
        let mut record = Map::from_iter([
            (String::from("id"), json!("record-id")),
            (String::from("name"), json!("Acme")),
            (String::from("hidden"), json!("value")),
        ]);

        pick_fields(&mut record, Some(&[String::from("name")]));
        assert_eq!(record.get("id"), Some(&json!("record-id")));
        assert_eq!(record.get("name"), Some(&json!("Acme")));
        assert!(record.get("hidden").is_none());
    }
}
