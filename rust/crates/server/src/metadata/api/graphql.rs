use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;

use super::errors::GraphQlError;

const NAME_START: &[u8] = b"_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NAME_REST_EXTRA: &[u8] = b"0123456789";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Query,
    Mutation,
}

impl OperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OperationKind::Query => "query",
            OperationKind::Mutation => "mutation",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSelection {
    pub alias: Option<String>,
    pub name: String,
    pub arguments: Vec<(String, Value)>,
    pub selection: Vec<FieldSelection>,
    pub spreads: Vec<String>,
}

impl FieldSelection {
    pub fn response_key(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }

    pub fn argument(&self, name: &str) -> Option<&Value> {
        self.arguments
            .iter()
            .find(|(argument, _)| argument == name)
            .map(|(_, value)| value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedOperation {
    pub kind: OperationKind,
    pub name: Option<String>,
    pub fields: Vec<FieldSelection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDocument {
    pub operations: Vec<ParsedOperation>,
    pub fragments: HashMap<String, Vec<FieldSelection>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionRequest {
    pub operation_name: Option<String>,
    pub variables: Value,
    pub document: Option<ParsedDocument>,
}

#[derive(Debug, Deserialize)]
pub struct GraphqlRequestBody {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(rename = "operationName", default)]
    pub operation_name: Option<String>,
    #[serde(default)]
    pub variables: Option<Value>,
    #[serde(default)]
    pub extensions: Option<Value>,
}

#[derive(Debug, Default)]
struct Scope {
    variables: Vec<(String, Value)>,
    fields: Vec<FieldSelection>,
    spreads: Vec<String>,
}

pub fn parse_request(body: GraphqlRequestBody) -> Result<ExecutionRequest, GraphQlError> {
    let variables = match body.variables {
        Some(Value::Object(_)) | None => body.variables.unwrap_or(Value::Null),
        Some(_) => {
            return Err(GraphQlError::validation(
                "Variables expected to be an Object.",
            ))
        }
    };

    let document = match &body.query {
        Some(query) => Some(parse_document(query, &variables)?),
        None => None,
    };

    Ok(ExecutionRequest {
        operation_name: body.operation_name,
        variables,
        document,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedOperation {
    pub kind: OperationKind,
    pub field: FieldSelection,
}

pub fn select_operation(request: &ExecutionRequest) -> Result<SelectedOperation, GraphQlError> {
    let document = request
        .document
        .as_ref()
        .ok_or_else(|| GraphQlError::validation("Must provide query string."))?;

    if document.operations.is_empty() {
        return Err(GraphQlError::validation(
            "Must provide an operation to execute.",
        ));
    }

    let matched: Vec<&ParsedOperation> = match &request.operation_name {
        Some(name) => document
            .operations
            .iter()
            .filter(|operation| operation.name.as_deref() == Some(name.as_str()))
            .collect(),
        None if document.operations.len() == 1 => document.operations.iter().collect(),
        None => {
            return Err(GraphQlError::validation(
                "Must provide operation name if query contains multiple operations.",
            ))
        }
    };

    let operation = matched.first().ok_or_else(|| {
        GraphQlError::validation(format!(
            "Unknown operation named \"{}\".",
            request.operation_name.clone().unwrap_or_default()
        ))
    })?;

    if operation.fields.len() > 1 {
        return Err(GraphQlError::validation(format!(
            "Operation \"{}\" has more than one root field.",
            request
                .operation_name
                .clone()
                .unwrap_or_else(|| operation.kind.as_str().to_string())
        )));
    }

    let field = operation
        .fields
        .first()
        .cloned()
        .ok_or_else(|| GraphQlError::parse("Syntax Error: Expected Name, found \"}\"."))?;

    Ok(SelectedOperation {
        kind: operation.kind,
        field,
    })
}

fn parse_document(query: &str, variables: &Value) -> Result<ParsedDocument, GraphQlError> {
    let mut parser = Parser::new(query);

    parser.parse_definitions(variables)
}

struct Parser<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_insignificant(&mut self) {
        loop {
            match self.peek_byte() {
                Some(byte) if byte.is_ascii_whitespace() || byte == b',' => {
                    self.advance();
                }
                Some(b'#') => {
                    while let Some(byte) = self.peek_byte() {
                        self.advance();

                        if byte == b'\n' || byte == b'\r' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn expect_byte(&mut self, byte: u8) -> Result<(), GraphQlError> {
        self.skip_insignificant();

        if self.peek_byte() == Some(byte) {
            self.advance();

            return Ok(());
        }

        Err(GraphQlError::parse(format!(
            "Syntax Error: Expected \"{}\", found {} at position {}.",
            byte as char,
            self.describe_current(),
            self.position
        )))
    }

    fn peek_byte_after_skipping(&mut self) -> Option<u8> {
        self.skip_insignificant();

        self.peek_byte()
    }

    fn describe_current(&self) -> String {
        match self.peek_byte() {
            None => "<EOF>".to_string(),
            Some(byte) => format!("\"{}\"", byte as char),
        }
    }

    fn starts_with(&self, text: &str) -> bool {
        self.input[self.position..].starts_with(text.as_bytes())
    }

    fn is_word_char(byte: u8) -> bool {
        NAME_START.contains(&byte) || NAME_REST_EXTRA.contains(&byte)
    }

    fn read_name(&mut self) -> Result<String, GraphQlError> {
        self.skip_insignificant();

        let start = self.position;

        match self.peek_byte() {
            Some(byte) if NAME_START.contains(&byte) => self.advance(),
            _ => {
                return Err(GraphQlError::parse(format!(
                    "Syntax Error: Expected Name, found {} at position {start}.",
                    self.describe_current()
                )))
            }
        }

        while let Some(byte) = self.peek_byte() {
            if NAME_START.contains(&byte) || NAME_REST_EXTRA.contains(&byte) {
                self.advance();
            } else {
                break;
            }
        }

        match std::str::from_utf8(&self.input[start..self.position]) {
            Ok(name) => Ok(name.to_string()),
            Err(_) => Err(GraphQlError::parse("Syntax Error: Invalid UTF-8 in name.")),
        }
    }

    fn parse_definitions(&mut self, variables: &Value) -> Result<ParsedDocument, GraphQlError> {
        let mut operations: Vec<ParsedOperation> = Vec::new();
        let mut fragments: HashMap<String, Vec<FieldSelection>> = HashMap::new();

        loop {
            self.skip_insignificant();

            if self.peek_byte().is_none() {
                break;
            }

            if self.starts_with("fragment")
                && self
                    .input
                    .get(self.position + "fragment".len())
                    .is_none_or(|byte| !Self::is_word_char(*byte))
            {
                self.parse_fragment_definition(&mut fragments, variables)?;
                continue;
            }

            let operation = if self.peek_byte() == Some(b'{') {
                let mut scope = Scope::default();

                self.selection_set(&mut scope, variables)?;

                ParsedOperation {
                    kind: OperationKind::Query,
                    name: None,
                    fields: scope.fields,
                }
            } else {
                self.parse_operation_definition(variables)?
            };

            operations.push(operation);
        }

        if operations.is_empty() && fragments.is_empty() {
            return Err(GraphQlError::parse(
                "Syntax Error: Expected Name, found <EOF>.",
            ));
        }

        if operations.is_empty() {
            return Err(GraphQlError::validation(
                "Must provide an operation to execute.",
            ));
        }

        for operation in &mut operations {
            let resolved = resolve_spreads(&operation.fields, &fragments, 0)?;

            operation.fields = resolved;
        }

        Ok(ParsedDocument {
            operations,
            fragments,
        })
    }

    fn parse_fragment_definition(
        &mut self,
        fragments: &mut HashMap<String, Vec<FieldSelection>>,
        variables: &Value,
    ) -> Result<(), GraphQlError> {
        for _ in 0.."fragment".len() {
            self.advance();
        }

        let name = self.read_name()?;

        self.skip_insignificant();

        if self.starts_with("on")
            && self
                .input
                .get(self.position + 2)
                .is_none_or(|byte| !Self::is_word_char(*byte))
        {
            self.advance();
            self.advance();
            self.read_name()?;
        }

        self.skip_directives()?;

        let mut scope = Scope::default();

        self.selection_set(&mut scope, variables)?;

        fragments.insert(name, scope.fields);

        Ok(())
    }

    fn parse_operation_definition(
        &mut self,
        variables: &Value,
    ) -> Result<ParsedOperation, GraphQlError> {
        let keyword = self.read_name()?;

        let kind = match keyword.as_str() {
            "query" => OperationKind::Query,
            "mutation" => OperationKind::Mutation,
            "subscription" => {
                return Err(GraphQlError::validation(
                    "Subscriptions are not supported by the metadata API.",
                ))
            }
            _ => {
                return Err(GraphQlError::parse(format!(
                    "Syntax Error: Unexpected {keyword} at position {}.",
                    self.position
                )))
            }
        };

        let mut scope = Scope::default();

        let mut name = None;

        if self
            .peek_byte_after_skipping()
            .is_some_and(|byte| NAME_START.contains(&byte))
        {
            name = Some(self.read_name()?);
        }

        if self.peek_byte_after_skipping() == Some(b'(') {
            self.advance();
            self.parse_variable_definitions(&mut scope, variables)?;
        }

        self.skip_directives()?;

        self.selection_set(&mut scope, variables)?;

        Ok(ParsedOperation {
            kind,
            name,
            fields: scope.fields,
        })
    }

    fn parse_variable_definitions(
        &mut self,
        scope: &mut Scope,
        variables: &Value,
    ) -> Result<(), GraphQlError> {
        loop {
            if self.peek_byte_after_skipping() == Some(b')') {
                self.advance();

                return Ok(());
            }

            self.expect_byte(b'$')?;

            let name = self.read_name()?;

            self.expect_byte(b':')?;
            self.parse_type()?;

            self.skip_directives()?;

            if self.peek_byte_after_skipping() == Some(b'=') {
                self.advance();

                let default = self.parse_value(variables, true, &scope.variables)?;

                scope.variables.push((name, default));
            }
        }
    }

    fn parse_type(&mut self) -> Result<(), GraphQlError> {
        self.skip_insignificant();

        if self.peek_byte() == Some(b'[') {
            self.advance();
            self.parse_type()?;
            self.expect_byte(b']')?;
        } else {
            self.read_name()?;
        }

        if self.peek_byte_after_skipping() == Some(b'!') {
            self.advance();
        }

        Ok(())
    }

    fn skip_directives(&mut self) -> Result<(), GraphQlError> {
        loop {
            if self.peek_byte_after_skipping() != Some(b'@') {
                return Ok(());
            }

            self.advance();
            self.read_name()?;

            if self.peek_byte_after_skipping() == Some(b'(') {
                self.parse_arguments(&Value::Null, true, &Vec::new())?;
            }
        }
    }

    fn selection_set(&mut self, scope: &mut Scope, variables: &Value) -> Result<(), GraphQlError> {
        self.expect_byte(b'{')?;

        loop {
            self.skip_insignificant();

            match self.peek_byte() {
                Some(b'}') => {
                    self.advance();

                    return Ok(());
                }
                None => return Err(GraphQlError::parse("Syntax Error: Unexpected <EOF>.")),
                Some(b'.') => {
                    self.expect_byte(b'.')?;
                    self.expect_byte(b'.')?;
                    self.expect_byte(b'.')?;

                    if self
                        .peek_byte_after_skipping()
                        .is_some_and(|byte| byte != b'{')
                    {
                        if self.starts_with("on")
                            && self
                                .input
                                .get(self.position + 2)
                                .is_none_or(|byte| !Self::is_word_char(*byte))
                        {
                            self.advance();
                            self.advance();
                            self.read_name()?;
                            self.skip_directives()?;
                            self.selection_set(scope, variables)?;

                            continue;
                        }

                        let fragment_name = self.read_name()?;

                        scope.spreads.push(fragment_name);
                        continue;
                    }

                    self.skip_directives()?;
                    self.selection_set(scope, variables)?;
                }
                _ => {
                    let field = self.parse_field(scope, variables)?;

                    scope.fields.push(field);
                }
            }
        }
    }

    fn parse_field(
        &mut self,
        scope: &mut Scope,
        variables: &Value,
    ) -> Result<FieldSelection, GraphQlError> {
        let mut alias = None;

        let mut name = self.read_name()?;

        if self.peek_byte_after_skipping() == Some(b':') {
            self.advance();
            alias = Some(name);
            name = self.read_name()?;
        }

        let mut arguments: Vec<(String, Value)> = Vec::new();

        if self.peek_byte_after_skipping() == Some(b'(') {
            arguments = self.parse_arguments(variables, false, &scope.variables)?;
        }

        self.skip_directives()?;

        let mut selection = Vec::new();
        let mut spreads = Vec::new();

        if self.peek_byte_after_skipping() == Some(b'{') {
            let mut nested_scope = Scope {
                variables: scope.variables.clone(),
                fields: Vec::new(),
                spreads: Vec::new(),
            };

            self.selection_set(&mut nested_scope, variables)?;

            selection = nested_scope.fields;
            spreads = nested_scope.spreads;
        }

        Ok(FieldSelection {
            alias,
            name,
            arguments,
            selection,
            spreads,
        })
    }

    fn parse_arguments(
        &mut self,
        variables: &Value,
        allow_unresolved: bool,
        scope_variables: &[(String, Value)],
    ) -> Result<Vec<(String, Value)>, GraphQlError> {
        self.expect_byte(b'(')?;

        let mut arguments = Vec::new();

        loop {
            self.skip_insignificant();

            match self.peek_byte() {
                Some(b')') => {
                    self.advance();

                    return Ok(arguments);
                }
                None => return Err(GraphQlError::parse("Syntax Error: Unexpected <EOF>.")),
                _ => {}
            }

            let name = self.read_name()?;

            self.expect_byte(b':')?;

            let value = self.parse_value(variables, allow_unresolved, scope_variables)?;

            arguments.push((name, value));
        }
    }

    fn parse_value(
        &mut self,
        variables: &Value,
        allow_unresolved: bool,
        scope_variables: &[(String, Value)],
    ) -> Result<Value, GraphQlError> {
        self.skip_insignificant();

        match self.peek_byte() {
            Some(b'$') => {
                self.advance();

                let name = self.read_name()?;

                if let Some((_, value)) = scope_variables
                    .iter()
                    .find(|(candidate, _)| *candidate == name)
                {
                    return Ok(value.clone());
                }

                let resolved = variables.get(&name).cloned();

                match resolved {
                    Some(value) => Ok(value),
                    None if allow_unresolved => Ok(Value::Null),
                    None => Err(GraphQlError::validation(format!(
                        "Variable \"${name}\" is not defined."
                    ))),
                }
            }
            Some(b'[') => {
                self.advance();

                let mut items = Vec::new();

                loop {
                    self.skip_insignificant();

                    if self.peek_byte() == Some(b']') {
                        self.advance();

                        return Ok(Value::Array(items));
                    }

                    items.push(self.parse_value(variables, allow_unresolved, scope_variables)?);
                }
            }
            Some(b'{') => {
                self.advance();

                let mut object = serde_json::Map::new();

                loop {
                    self.skip_insignificant();

                    if self.peek_byte() == Some(b'}') {
                        self.advance();

                        return Ok(Value::Object(object));
                    }

                    let name = self.read_name()?;

                    self.expect_byte(b':')?;

                    let value = self.parse_value(variables, allow_unresolved, scope_variables)?;

                    object.insert(name, value);
                }
            }
            Some(b'"') => self.parse_string(),
            Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.parse_number(),
            Some(byte) if NAME_START.contains(&byte) => {
                let name = self.read_name()?;

                match name.as_str() {
                    "true" => Ok(json!(true)),
                    "false" => Ok(json!(false)),
                    "null" => Ok(Value::Null),
                    _ => Ok(Value::String(name)),
                }
            }
            _ => Err(GraphQlError::parse(format!(
                "Syntax Error: Unexpected {} at position {}.",
                self.describe_current(),
                self.position
            ))),
        }
    }

    fn parse_string(&mut self) -> Result<Value, GraphQlError> {
        self.expect_byte(b'"')?;

        let mut result = String::new();

        loop {
            let byte = match self.peek_byte() {
                Some(byte) => byte,
                None => return Err(GraphQlError::parse("Syntax Error: Unterminated string.")),
            };

            self.advance();

            match byte {
                b'"' => return Ok(Value::String(result)),
                b'\\' => {
                    let escape = match self.peek_byte() {
                        Some(byte) => byte,
                        None => {
                            return Err(GraphQlError::parse("Syntax Error: Unterminated escape."))
                        }
                    };

                    self.advance();

                    match escape {
                        b'"' => result.push('"'),
                        b'\\' => result.push('\\'),
                        b'/' => result.push('/'),
                        b'b' => result.push('\u{0008}'),
                        b'f' => result.push('\u{000C}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            let mut code = 0u32;

                            for _ in 0..4 {
                                let digit = match self.peek_byte() {
                                    Some(byte) => byte,
                                    None => {
                                        return Err(GraphQlError::parse(
                                            "Syntax Error: Invalid unicode escape.",
                                        ))
                                    }
                                };

                                self.advance();

                                let value = (digit as char).to_digit(16).ok_or_else(|| {
                                    GraphQlError::parse("Syntax Error: Invalid unicode escape.")
                                })?;

                                code = code * 16 + value;
                            }

                            let character = char::from_u32(code).ok_or_else(|| {
                                GraphQlError::parse("Syntax Error: Invalid unicode escape.")
                            })?;

                            result.push(character);
                        }
                        _ => {
                            return Err(GraphQlError::parse(
                                "Syntax Error: Invalid escape sequence.",
                            ))
                        }
                    }
                }
                _ => {
                    let start = self.position - 1;

                    let extra = utf8_sequence_length(byte);

                    for _ in 1..extra {
                        self.advance();
                    }

                    let end = self.position;

                    let slice = std::str::from_utf8(&self.input[start..end]).map_err(|_| {
                        GraphQlError::parse("Syntax Error: Invalid UTF-8 in string.")
                    })?;

                    result.push_str(slice);
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<Value, GraphQlError> {
        let start = self.position;

        if self.peek_byte() == Some(b'-') {
            self.advance();
        }

        while let Some(byte) = self.peek_byte() {
            let previous = self
                .position
                .checked_sub(1)
                .and_then(|index| self.input.get(index));

            let continues = byte.is_ascii_digit()
                || byte == b'.'
                || byte == b'e'
                || byte == b'E'
                || ((byte == b'+' || byte == b'-')
                    && previous.is_some_and(|value| *value == b'e' || *value == b'E'));

            if continues {
                self.advance();
            } else {
                break;
            }
        }

        let raw = std::str::from_utf8(&self.input[start..self.position])
            .map_err(|_| GraphQlError::parse("Syntax Error: Invalid number."))?;

        if let Ok(integer) = raw.parse::<i64>() {
            return Ok(json!(integer));
        }

        raw.parse::<f64>()
            .map(|value| json!(value))
            .map_err(|_| GraphQlError::parse(format!("Syntax Error: Invalid number {raw}.")))
    }
}

fn resolve_spreads(
    fields: &[FieldSelection],
    fragments: &HashMap<String, Vec<FieldSelection>>,
    depth: usize,
) -> Result<Vec<FieldSelection>, GraphQlError> {
    if depth > 8 {
        return Err(GraphQlError::validation(
            "Too many nested fragment spreads.",
        ));
    }

    let mut resolved = Vec::new();

    for field in fields {
        let mut clone = field.clone();

        clone.selection = resolve_spreads(&field.selection, fragments, depth + 1)?;

        for spread in &field.spreads {
            let Some(fragment_fields) = fragments.get(spread) else {
                return Err(GraphQlError::validation(format!(
                    "Unknown fragment \"{spread}\"."
                )));
            };

            let expanded = resolve_spreads(fragment_fields, fragments, depth + 1)?;

            for fragment_field in expanded {
                if !clone
                    .selection
                    .iter()
                    .any(|existing| existing.response_key() == fragment_field.response_key())
                {
                    clone.selection.push(fragment_field);
                }
            }
        }

        resolved.push(clone);
    }

    Ok(resolved)
}

fn utf8_sequence_length(first_byte: u8) -> usize {
    if first_byte < 0x80 {
        1
    } else if first_byte >= 0xF0 {
        4
    } else if first_byte >= 0xE0 {
        3
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parse(query: &str, variables: Value) -> Result<ParsedDocument, GraphQlError> {
        parse_document(query, &variables)
    }

    #[test]
    fn parses_mutation_with_inline_arguments() {
        let document = parse(
            "mutation { createOneObject(input: { object: { nameSingular: \"rocket\" } }) { id } }",
            json!({}),
        )
        .unwrap();

        assert_eq!(document.operations.len(), 1);
        assert_eq!(document.operations[0].kind, OperationKind::Mutation);
        assert_eq!(document.operations[0].fields.len(), 1);
        assert_eq!(document.operations[0].fields[0].name, "createOneObject");
        assert_eq!(
            document.operations[0].fields[0].argument("input"),
            Some(&json!({ "object": { "nameSingular": "rocket" } }))
        );
        assert_eq!(document.operations[0].fields[0].selection[0].name, "id");
    }

    #[test]
    fn parses_operation_with_variables_and_default() {
        let document = parse(
            "mutation Create($input: CreateOneObjectInput, $limit: Int = 5) { updateOneObject(input: $input, limit: $limit) { id } }",
            json!({ "input": { "id": "one" } }),
        )
        .unwrap();

        assert_eq!(document.operations[0].name.as_deref(), Some("Create"));
        assert_eq!(document.operations[0].fields[0].name, "updateOneObject");
        assert_eq!(
            document.operations[0].fields[0].argument("input"),
            Some(&json!({ "id": "one" }))
        );
        assert_eq!(
            document.operations[0].fields[0].argument("limit"),
            Some(&json!(5))
        );
    }

    #[test]
    fn anonymous_query_is_supported() {
        let document = parse(
            "{ objects(filter: { isActive: { eq: true } }) { edges { node { id } } } }",
            json!({}),
        )
        .unwrap();

        assert_eq!(document.operations[0].kind, OperationKind::Query);
        assert_eq!(document.operations[0].fields[0].name, "objects");
    }

    #[test]
    fn aliases_are_tracked() {
        let document = parse(
            "query Q { first: objects { edges { node { id } } } }",
            json!({}),
        )
        .unwrap();

        assert_eq!(
            document.operations[0].fields[0].alias.as_deref(),
            Some("first")
        );
        assert_eq!(document.operations[0].fields[0].response_key(), "first");
    }

    #[test]
    fn enum_and_list_literals_are_captured() {
        let document = parse(
            "mutation { createOneIndex(input: { objectMetadataId: \"u\", fields: [{ fieldMetadataId: \"f\" }], indexType: GIN }) { id } }",
            json!({}),
        )
        .unwrap();

        assert_eq!(
            document.operations[0].fields[0].argument("input"),
            Some(&json!({
                "objectMetadataId": "u",
                "fields": [{ "fieldMetadataId": "f" }],
                "indexType": "GIN"
            }))
        );
    }

    #[test]
    fn strings_with_escapes_are_decoded() {
        let document = parse(
            "mutation { deleteOneObject(input: { id: \"a\\u0062c\" }) { id } }",
            json!({}),
        )
        .unwrap();

        assert_eq!(
            document.operations[0].fields[0].argument("input"),
            Some(&json!({ "id": "abc" }))
        );
    }

    #[test]
    fn comments_and_commas_are_insignificant() {
        let document = parse(
            "query X {, objects { edges { node { id, } }, }, }",
            json!({}),
        )
        .unwrap();

        assert_eq!(document.operations[0].fields[0].name, "objects");
    }

    #[test]
    fn variable_without_value_is_undefined() {
        let error = parse(
            "query Q { objects(filter: $missing) { edges { node { id } } } }",
            json!({}),
        )
        .expect_err("undefined variables must fail");

        assert_eq!(error.code, "GRAPHQL_VALIDATION_FAILED");
    }

    #[test]
    fn named_fragment_spreads_are_expanded() {
        let document = parse(
            "query Q { objects { edges { node { ...ObjectParts } } } } fragment ObjectParts on ObjectMetadataDTO { id nameSingular }",
            json!({}),
        )
        .unwrap();

        let node = &document.operations[0].fields[0].selection[0].selection[0];

        assert_eq!(node.name, "node");

        let names: Vec<&str> = node
            .selection
            .iter()
            .map(|field| field.name.as_str())
            .collect();

        assert_eq!(names, vec!["id", "nameSingular"]);
    }

    #[test]
    fn inline_fragments_merge_selections() {
        let document = parse(
            "query Q { objects { edges { node { ... { id } } } } }",
            json!({}),
        )
        .unwrap();

        let node = &document.operations[0].fields[0].selection[0].selection[0];

        assert_eq!(node.selection[0].name, "id");
    }

    #[test]
    fn unknown_operation_name_is_reported() {
        let body = GraphqlRequestBody {
            query: Some("query A { objects { edges { node { id } } } } query B { fields { edges { node { id } } } }".to_string()),
            operation_name: Some("C".to_string()),
            variables: None,
            extensions: None,
        };

        let request = parse_request(body).unwrap();
        let error = select_operation(&request).expect_err("unknown operation");

        assert_eq!(error.code, "GRAPHQL_VALIDATION_FAILED");
        assert!(error.message.contains("Unknown operation named \"C\""));
    }

    #[test]
    fn multiple_operations_require_operation_name() {
        let body = GraphqlRequestBody {
            query: Some("query A { objects { edges { node { id } } } } query B { fields { edges { node { id } } } }".to_string()),
            operation_name: None,
            variables: None,
            extensions: None,
        };

        let request = parse_request(body).unwrap();
        let error = select_operation(&request).expect_err("must name operation");

        assert!(error.message.contains("multiple operations"));
    }

    #[test]
    fn operation_name_selects_the_named_operation() {
        let body = GraphqlRequestBody {
            query: Some("query A { objects { edges { node { id } } } } query B { fields { edges { node { id } } } }".to_string()),
            operation_name: Some("B".to_string()),
            variables: None,
            extensions: None,
        };

        let request = parse_request(body).unwrap();
        let selected = select_operation(&request).unwrap();

        assert_eq!(selected.kind, OperationKind::Query);
        assert_eq!(selected.field.name, "fields");
    }

    #[test]
    fn subscription_is_refused() {
        let error = parse(
            "subscription S { objects { edges { node { id } } } }",
            json!({}),
        )
        .expect_err("subscriptions unsupported");

        assert_eq!(error.code, "GRAPHQL_VALIDATION_FAILED");
    }

    #[test]
    fn empty_query_string_is_reported() {
        let error = parse("", json!({})).expect_err("empty document");

        assert_eq!(error.code, "GRAPHQL_PARSE_FAILED");
    }

    #[test]
    fn request_body_variables_must_be_an_object() {
        let body = GraphqlRequestBody {
            query: None,
            operation_name: None,
            variables: Some(json!([1, 2])),
            extensions: None,
        };

        let error = parse_request(body).expect_err("non-object variables");

        assert_eq!(error.code, "GRAPHQL_VALIDATION_FAILED");
    }
}
