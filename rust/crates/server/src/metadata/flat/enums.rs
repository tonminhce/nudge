#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldMetadataType {
    #[serde(rename = "ACTOR")]
    Actor,
    #[serde(rename = "ADDRESS")]
    Address,
    #[serde(rename = "ARRAY")]
    Array,
    #[serde(rename = "BOOLEAN")]
    Boolean,
    #[serde(rename = "CURRENCY")]
    Currency,
    #[serde(rename = "DATE")]
    Date,
    #[serde(rename = "DATE_TIME")]
    DateTime,
    #[serde(rename = "EMAILS")]
    Emails,
    #[serde(rename = "FILES")]
    Files,
    #[serde(rename = "FULL_NAME")]
    FullName,
    #[serde(rename = "LINKS")]
    Links,
    #[serde(rename = "MORPH_RELATION")]
    MorphRelation,
    #[serde(rename = "MULTI_SELECT")]
    MultiSelect,
    #[serde(rename = "NUMBER")]
    Number,
    #[serde(rename = "NUMERIC")]
    Numeric,
    #[serde(rename = "PHONES")]
    Phones,
    #[serde(rename = "POSITION")]
    Position,
    #[serde(rename = "RATING")]
    Rating,
    #[serde(rename = "RAW_JSON")]
    RawJson,
    #[serde(rename = "RELATION")]
    Relation,
    #[serde(rename = "RICH_TEXT")]
    RichText,
    #[serde(rename = "SELECT")]
    Select,
    #[serde(rename = "TEXT")]
    Text,
    #[serde(rename = "TS_VECTOR")]
    TsVector,
    #[serde(rename = "UUID")]
    Uuid,
}

impl FieldMetadataType {
    pub fn as_str(self) -> &'static str {
        match self {
            FieldMetadataType::Actor => "ACTOR",
            FieldMetadataType::Address => "ADDRESS",
            FieldMetadataType::Array => "ARRAY",
            FieldMetadataType::Boolean => "BOOLEAN",
            FieldMetadataType::Currency => "CURRENCY",
            FieldMetadataType::Date => "DATE",
            FieldMetadataType::DateTime => "DATE_TIME",
            FieldMetadataType::Emails => "EMAILS",
            FieldMetadataType::Files => "FILES",
            FieldMetadataType::FullName => "FULL_NAME",
            FieldMetadataType::Links => "LINKS",
            FieldMetadataType::MorphRelation => "MORPH_RELATION",
            FieldMetadataType::MultiSelect => "MULTI_SELECT",
            FieldMetadataType::Number => "NUMBER",
            FieldMetadataType::Numeric => "NUMERIC",
            FieldMetadataType::Phones => "PHONES",
            FieldMetadataType::Position => "POSITION",
            FieldMetadataType::Rating => "RATING",
            FieldMetadataType::RawJson => "RAW_JSON",
            FieldMetadataType::Relation => "RELATION",
            FieldMetadataType::RichText => "RICH_TEXT",
            FieldMetadataType::Select => "SELECT",
            FieldMetadataType::Text => "TEXT",
            FieldMetadataType::TsVector => "TS_VECTOR",
            FieldMetadataType::Uuid => "UUID",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "ACTOR" => FieldMetadataType::Actor,
            "ADDRESS" => FieldMetadataType::Address,
            "ARRAY" => FieldMetadataType::Array,
            "BOOLEAN" => FieldMetadataType::Boolean,
            "CURRENCY" => FieldMetadataType::Currency,
            "DATE" => FieldMetadataType::Date,
            "DATE_TIME" => FieldMetadataType::DateTime,
            "EMAILS" => FieldMetadataType::Emails,
            "FILES" => FieldMetadataType::Files,
            "FULL_NAME" => FieldMetadataType::FullName,
            "LINKS" => FieldMetadataType::Links,
            "MORPH_RELATION" => FieldMetadataType::MorphRelation,
            "MULTI_SELECT" => FieldMetadataType::MultiSelect,
            "NUMBER" => FieldMetadataType::Number,
            "NUMERIC" => FieldMetadataType::Numeric,
            "PHONES" => FieldMetadataType::Phones,
            "POSITION" => FieldMetadataType::Position,
            "RATING" => FieldMetadataType::Rating,
            "RAW_JSON" => FieldMetadataType::RawJson,
            "RELATION" => FieldMetadataType::Relation,
            "RICH_TEXT" => FieldMetadataType::RichText,
            "SELECT" => FieldMetadataType::Select,
            "TEXT" => FieldMetadataType::Text,
            "TS_VECTOR" => FieldMetadataType::TsVector,
            "UUID" => FieldMetadataType::Uuid,
            _ => return None,
        })
    }

    pub fn is_relation_or_morph(self) -> bool {
        matches!(
            self,
            FieldMetadataType::Relation | FieldMetadataType::MorphRelation
        )
    }

    pub fn is_enum_type(self) -> bool {
        matches!(
            self,
            FieldMetadataType::Select | FieldMetadataType::MultiSelect | FieldMetadataType::Rating
        )
    }

    pub fn is_composite(self) -> bool {
        matches!(
            self,
            FieldMetadataType::Actor
                | FieldMetadataType::Address
                | FieldMetadataType::Currency
                | FieldMetadataType::Emails
                | FieldMetadataType::Files
                | FieldMetadataType::FullName
                | FieldMetadataType::Links
                | FieldMetadataType::Phones
                | FieldMetadataType::RichText
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectOpenRecordIn {
    #[serde(rename = "SIDE_PANEL")]
    SidePanel,
    #[serde(rename = "RECORD_PAGE")]
    RecordPage,
    #[serde(rename = "USER_CHOICE")]
    UserChoice,
}

impl ObjectOpenRecordIn {
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectOpenRecordIn::SidePanel => "SIDE_PANEL",
            ObjectOpenRecordIn::RecordPage => "RECORD_PAGE",
            ObjectOpenRecordIn::UserChoice => "USER_CHOICE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IndexType {
    #[default]
    #[serde(rename = "BTREE")]
    Btree,
    #[serde(rename = "GIN")]
    Gin,
}

impl IndexType {
    pub fn as_str(self) -> &'static str {
        match self {
            IndexType::Btree => "BTREE",
            IndexType::Gin => "GIN",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "BTREE" => IndexType::Btree,
            "GIN" => IndexType::Gin,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    OneToMany,
    ManyToOne,
    ManyToMany,
}

impl RelationType {
    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "ONE_TO_MANY" => RelationType::OneToMany,
            "MANY_TO_ONE" => RelationType::ManyToOne,
            "MANY_TO_MANY" => RelationType::ManyToMany,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FieldMetadataDefaultOption {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub position: f64,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FieldMetadataComplexOption {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub position: f64,
    #[serde(default)]
    pub value: serde_json::Value,
}
