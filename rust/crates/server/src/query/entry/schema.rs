use std::collections::HashMap;

use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatObjectMetadata;
use crate::metadata::flat::FlatWorkspaceMaps;
use crate::metadata::names::camel_case;
use crate::metadata::names::pascal_case;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverMethod {
    FindMany,
    FindOne,
    CreateOne,
    CreateMany,
    UpdateOne,
    DeleteOne,
    DestroyOne,
    RestoreOne,
    GroupBy,
}

impl ResolverMethod {
    pub fn is_query(self) -> bool {
        matches!(self, Self::FindMany | Self::FindOne | Self::GroupBy)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FindMany => "findMany",
            Self::FindOne => "findOne",
            Self::CreateOne => "createOne",
            Self::CreateMany => "createMany",
            Self::UpdateOne => "updateOne",
            Self::DeleteOne => "deleteOne",
            Self::DestroyOne => "destroyOne",
            Self::RestoreOne => "restoreOne",
            Self::GroupBy => "groupBy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootResolver {
    pub method: ResolverMethod,
    pub object_name_singular: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSchema {
    objects: Vec<FlatObjectMetadata>,
    fields_by_object_id: HashMap<String, Vec<FlatFieldMetadata>>,
    resolvers_by_root: HashMap<String, RootResolver>,
}

impl WorkspaceSchema {
    pub fn build(maps: &FlatWorkspaceMaps) -> Self {
        let mut objects = maps
            .object_metadata_maps
            .by_universal_identifier
            .values()
            .cloned()
            .collect::<Vec<_>>();
        objects.sort_by(|left, right| left.name_singular.cmp(&right.name_singular));

        let mut fields_by_object_id = HashMap::new();

        for object in &objects {
            let fields = object
                .field_ids
                .iter()
                .filter_map(|field_id| maps.field_metadata_maps.find_by_id(field_id))
                .cloned()
                .collect::<Vec<_>>();

            fields_by_object_id.insert(object.id.to_string(), fields);
        }

        let mut resolvers_by_root = HashMap::new();

        for object in &objects {
            for (method, root) in [
                (ResolverMethod::FindMany, camel_case(&object.name_plural)),
                (ResolverMethod::FindOne, camel_case(&object.name_singular)),
                (
                    ResolverMethod::CreateOne,
                    format!("create{}", pascal_case(&object.name_singular)),
                ),
                (
                    ResolverMethod::CreateMany,
                    format!("create{}", pascal_case(&object.name_plural)),
                ),
                (
                    ResolverMethod::UpdateOne,
                    format!("update{}", pascal_case(&object.name_singular)),
                ),
                (
                    ResolverMethod::DeleteOne,
                    format!("delete{}", pascal_case(&object.name_singular)),
                ),
                (
                    ResolverMethod::DestroyOne,
                    format!("destroy{}", pascal_case(&object.name_singular)),
                ),
                (
                    ResolverMethod::RestoreOne,
                    format!("restore{}", pascal_case(&object.name_singular)),
                ),
                (
                    ResolverMethod::GroupBy,
                    format!("{}GroupBy", camel_case(&object.name_plural)),
                ),
            ] {
                resolvers_by_root.insert(
                    root,
                    RootResolver {
                        method,
                        object_name_singular: object.name_singular.clone(),
                    },
                );
            }
        }

        Self {
            objects,
            fields_by_object_id,
            resolvers_by_root,
        }
    }

    pub fn resolver(&self, root_field: &str) -> Option<&RootResolver> {
        self.resolvers_by_root.get(root_field)
    }

    pub fn object(&self, name_singular: &str) -> Option<&FlatObjectMetadata> {
        self.objects
            .iter()
            .find(|object| object.name_singular == name_singular)
    }

    pub fn fields(&self, object_name_singular: &str) -> &[FlatFieldMetadata] {
        self.object(object_name_singular)
            .and_then(|object| self.fields_by_object_id.get(&object.id.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn objects(&self) -> &[FlatObjectMetadata] {
        &self.objects
    }

    pub fn query_root_fields(&self) -> Vec<String> {
        let mut fields = self
            .objects
            .iter()
            .flat_map(|object| {
                [
                    camel_case(&object.name_plural),
                    camel_case(&object.name_singular),
                ]
            })
            .collect::<Vec<_>>();
        fields.push(String::from("search"));
        fields
    }

    pub fn mutation_root_fields(&self) -> Vec<String> {
        self.objects
            .iter()
            .flat_map(|object| {
                [
                    format!("create{}", pascal_case(&object.name_singular)),
                    format!("create{}", pascal_case(&object.name_plural)),
                    format!("update{}", pascal_case(&object.name_singular)),
                    format!("delete{}", pascal_case(&object.name_singular)),
                    format!("destroy{}", pascal_case(&object.name_singular)),
                    format!("restore{}", pascal_case(&object.name_singular)),
                ]
            })
            .collect()
    }

    pub fn sdl(&self) -> String {
        let mut sdl = String::from("schema {\n  query: Query\n  mutation: Mutation\n}\n\n");
        sdl.push_str("scalar DateTime\nscalar JSON\nscalar RawJson\nscalar UUID\n\n");
        sdl.push_str("enum FilterIs {\n  NULL\n  NOT_NULL\n}\n\n");
        sdl.push_str("input UUIDFilter {\n  eq: UUID\n  gt: UUID\n  gte: UUID\n  in: [UUID]\n  lt: UUID\n  lte: UUID\n  neq: UUID\n  is: FilterIs\n}\n\n");
        sdl.push_str("input DateTimeFilter {\n  eq: DateTime\n  gt: DateTime\n  gte: DateTime\n  in: [DateTime!]\n  lt: DateTime\n  lte: DateTime\n  neq: DateTime\n  is: FilterIs\n  gtField: String\n  gteField: String\n  ltField: String\n  lteField: String\n}\n\n");
        sdl.push_str("input ObjectRecordFilterInput {\n  and: [ObjectRecordFilterInput]\n  not: ObjectRecordFilterInput\n  or: [ObjectRecordFilterInput]\n  id: UUIDFilter\n  createdAt: DateTimeFilter\n  updatedAt: DateTimeFilter\n  deletedAt: DateTimeFilter\n}\n\n");
        sdl.push_str("type PageInfo {\n  startCursor: String\n  endCursor: String\n  hasNextPage: Boolean!\n  hasPreviousPage: Boolean!\n}\n\n");

        for object in &self.objects {
            let type_name = pascal_case(&object.name_singular);
            let connection_name = format!("{type_name}Connection");
            let edge_name = format!("{type_name}Edge");

            sdl.push_str(&format!("type {type_name} {{\n"));
            sdl.push_str("  id: ID!\n");

            for field in self.fields(&object.name_singular) {
                if field.field_type == crate::metadata::flat::FieldMetadataType::Relation {
                    let relation_type = field
                        .settings
                        .as_ref()
                        .and_then(|settings| settings.get("relationType"))
                        .and_then(serde_json::Value::as_str);

                    if relation_type == Some("MANY_TO_ONE") {
                        let target = field
                            .relation_target_object_metadata_id
                            .and_then(|target_id| {
                                self.objects.iter().find(|object| object.id == target_id)
                            })
                            .map(|object| pascal_case(&object.name_singular))
                            .unwrap_or_else(|| "JSON".to_string());

                        sdl.push_str(&format!("  {}: {target}\n", field.name));
                        continue;
                    }

                    if relation_type == Some("ONE_TO_MANY") {
                        let target = field
                            .relation_target_object_metadata_id
                            .and_then(|target_id| {
                                self.objects.iter().find(|object| object.id == target_id)
                            })
                            .map(|object| pascal_case(&object.name_singular))
                            .unwrap_or_else(|| "JSON".to_string());

                        sdl.push_str(&format!("  {}: [{target}Connection!]\n", field.name));
                        continue;
                    }
                }

                if field.field_type == crate::metadata::flat::FieldMetadataType::MorphRelation {
                    sdl.push_str(&format!("  {}: JSON\n", field.name));
                    continue;
                }

                sdl.push_str(&format!(
                    "  {}: {}\n",
                    field.name,
                    graphql_scalar(field.field_type)
                ));
            }

            sdl.push_str("  createdBy: JSON\n  updatedBy: JSON\n}\n\n");
            sdl.push_str(&format!(
                "type {edge_name} {{\n  node: {type_name}!\n  cursor: String!\n}}\n\n"
            ));
            sdl.push_str(&format!(
                "type {connection_name} {{\n  edges: [{edge_name}!]\n  pageInfo: PageInfo!\n  totalCount: Int\n}}\n\n"
            ));
        }

        sdl.push_str("type SearchRecord {\n  recordId: UUID!\n  objectNameSingular: String!\n  objectLabelSingular: String!\n  label: String!\n  imageUrl: String\n  tsRankCD: Float!\n  tsRank: Float!\n}\n\n");
        sdl.push_str("type SearchResultEdge {\n  node: SearchRecord!\n  cursor: String!\n}\n\n");
        sdl.push_str(
            "type SearchResultPageInfo {\n  endCursor: String\n  hasNextPage: Boolean!\n}\n\n",
        );
        sdl.push_str("type SearchResultConnection {\n  edges: [SearchResultEdge!]!\n  pageInfo: SearchResultPageInfo!\n}\n\n");
        sdl.push_str("type Query {\n");
        sdl.push_str("  search(searchInput: String!, limit: Int!, after: String, includedObjectNameSingulars: [String!], filter: ObjectRecordFilterInput, excludedObjectNameSingulars: [String!]): SearchResultConnection!\n");

        for object in &self.objects {
            let type_name = pascal_case(&object.name_singular);
            let connection_name = format!("{type_name}Connection");
            let plural_camel = camel_case(&object.name_plural);
            let singular_camel = camel_case(&object.name_singular);

            sdl.push_str(&format!(
                "  {plural_camel}(first: Int, last: Int, offset: Int, before: String, after: String, filter: JSON, orderBy: [JSON!]): {connection_name}!\n"
            ));
            sdl.push_str(&format!(
                "  {singular_camel}(filter: JSON!): {type_name}!\n"
            ));
        }

        sdl.push_str("}\n\ntype Mutation {\n");

        for object in &self.objects {
            let type_name = pascal_case(&object.name_singular);
            let singular = pascal_case(&object.name_singular);
            let plural = pascal_case(&object.name_plural);

            for prefix in ["create", "update", "delete", "destroy", "restore"] {
                let name = format!("{prefix}{singular}");

                sdl.push_str(&format!(
                    "  {name}(data: JSON, upsert: Boolean, id: ID): {type_name}!\n"
                ));
            }

            let name = format!("create{plural}");
            sdl.push_str(&format!(
                "  {name}(data: [JSON!]!, upsert: Boolean): [{type_name}!]!\n"
            ));
        }

        sdl.push_str("}\n");
        sdl
    }
}

fn graphql_scalar(field_type: crate::metadata::flat::FieldMetadataType) -> &'static str {
    use crate::metadata::flat::FieldMetadataType::*;

    match field_type {
        Uuid => "ID",
        Boolean => "Boolean",
        Number | Numeric | Rating | Position => "Float",
        Date => "Date",
        DateTime => "DateTime",
        RawJson | Files => "RawJson",
        Array | MultiSelect => "[String!]",
        _ => "String",
    }
}

pub const INTROSPECTION_FIELDS: [&str; 2] = ["__schema", "__type"];

pub const CORE_FIELDS: [&str; 11] = [
    "currentUser",
    "currentWorkspace",
    "workspaceMember",
    "workspaceMembers",
    "user",
    "users",
    "findWorkspaceFromInviteHash",
    "validateWorkspaceInviteHash",
    "apiKey",
    "apiKeys",
    "__typename",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootClassification {
    Introspection,
    Workspace,
    Core,
}

pub fn classify_root(root_field: &str, schema: &WorkspaceSchema) -> RootClassification {
    if INTROSPECTION_FIELDS.contains(&root_field) {
        return RootClassification::Introspection;
    }

    if root_field == "search" || schema.resolver(root_field).is_some() {
        return RootClassification::Workspace;
    }

    RootClassification::Core
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::flat::FlatWorkspaceMaps;
    use serde_json::json;

    fn schema_with_person() -> WorkspaceSchema {
        let mut maps = FlatWorkspaceMaps::default();
        let object = FlatObjectMetadata {
            id: uuid::Uuid::from_u128(1),
            name_singular: String::from("person"),
            name_plural: String::from("people"),
            ..FlatObjectMetadata::default()
        };

        let universal_identifier = object.universal_identifier.clone();

        maps.object_metadata_maps.add(
            object.id.to_string(),
            object,
            universal_identifier,
            uuid::Uuid::nil().to_string(),
        );

        WorkspaceSchema::build(&maps)
    }

    #[test]
    fn resolver_names_match_the_upstream_get_resolver_name_contract() {
        let schema = schema_with_person();

        for (root, method) in [
            ("people", ResolverMethod::FindMany),
            ("person", ResolverMethod::FindOne),
            ("createPerson", ResolverMethod::CreateOne),
            ("createPeople", ResolverMethod::CreateMany),
            ("updatePerson", ResolverMethod::UpdateOne),
            ("deletePerson", ResolverMethod::DeleteOne),
            ("destroyPerson", ResolverMethod::DestroyOne),
            ("restorePerson", ResolverMethod::RestoreOne),
            ("peopleGroupBy", ResolverMethod::GroupBy),
        ] {
            let resolver = schema
                .resolver(root)
                .unwrap_or_else(|| panic!("resolver {root}"));

            assert_eq!(resolver.method, method, "{root}");
            assert_eq!(resolver.object_name_singular, "person");
        }
    }

    #[test]
    fn roots_are_classified_into_the_three_upstream_execution_paths() {
        let schema = schema_with_person();

        assert_eq!(
            classify_root("__schema", &schema),
            RootClassification::Introspection
        );
        assert_eq!(
            classify_root("__type", &schema),
            RootClassification::Introspection
        );
        assert_eq!(
            classify_root("people", &schema),
            RootClassification::Workspace
        );
        assert_eq!(
            classify_root("search", &schema),
            RootClassification::Workspace
        );
        assert_eq!(
            classify_root("currentUser", &schema),
            RootClassification::Core
        );
    }

    #[test]
    fn sdl_contains_the_workspace_object_and_connection_names() {
        let sdl = schema_with_person().sdl();

        assert!(sdl.contains("type Person {"));
        assert!(sdl.contains("type PersonEdge {"));
        assert!(sdl.contains("type PersonConnection {"));
        assert!(sdl.contains("people(first: Int"));
        assert!(sdl.contains("person(filter: JSON!"));
        assert!(sdl.contains("type SearchResultConnection"));
        assert!(sdl.contains("input ObjectRecordFilterInput"));
        assert!(sdl.contains("input UUIDFilter"));
        assert!(sdl.contains("input DateTimeFilter"));
        assert!(sdl.contains("enum FilterIs"));
        assert!(sdl.contains("search(searchInput: String!, limit: Int!"));
        assert!(sdl.contains("filter: ObjectRecordFilterInput"));
        assert!(sdl.contains("createPerson(data: JSON"));
    }

    #[test]
    fn introspection_classifications_are_stable() {
        let schema = schema_with_person();
        let value = json!({"roots": [
            format!("{:?}", classify_root("__schema", &schema)),
            format!("{:?}", classify_root("people", &schema)),
            format!("{:?}", classify_root("currentUser", &schema)),
        ]});

        assert_eq!(
            value["roots"],
            json!(["Introspection", "Workspace", "Core"])
        );
    }
}
