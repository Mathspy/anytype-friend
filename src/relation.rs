use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use chrono::NaiveDateTime;

use crate::{
    object::{Object, ObjectId},
    object_type::ObjectTypeId,
    pb::models::RelationFormat as InternalRelationFormat,
    prost_ext::{IntoProstValue, ProstConversionError, ProstStruct, TryFromProst},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationSpec {
    /// The name of the relation
    pub name: String,
    pub format: RelationFormat,
}

impl From<RelationSpec> for prost_types::Struct {
    fn from(value: RelationSpec) -> Self {
        let mut fields = BTreeMap::from([
            ("name".to_string(), value.name.to_string().into_prost()),
            (
                "relationFormat".to_string(),
                f64::from(&value.format).into_prost(),
            ),
        ]);

        if let RelationFormat::Object { types } = value.format {
            fields.insert(
                "relationFormatObjectTypes".to_string(),
                types
                    .into_iter()
                    .map(|object_id| object_id.into_prost())
                    .collect::<Vec<_>>()
                    .into_prost(),
            );
        }

        prost_types::Struct { fields }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationFormat {
    Text,
    Number,
    Select,
    MultiSelect,
    Date,
    FileOrMedia,
    Checkbox,
    Url,
    Email,
    Phone,
    Object { types: BTreeSet<ObjectTypeId> },
}

impl RelationFormat {
    pub(crate) fn is_superset(&self, other: &RelationFormat) -> bool {
        match (self, other) {
            (
                RelationFormat::Object { types: self_types },
                RelationFormat::Object { types: other_types },
            ) => {
                if self_types.is_empty() {
                    true
                } else {
                    self_types.is_superset(other_types)
                }
            }
            (a, b) => a == b,
        }
    }
}

impl Display for RelationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationFormat::Text => f.write_str("Text"),
            RelationFormat::Number => f.write_str("Number"),
            RelationFormat::Select => f.write_str("Select"),
            RelationFormat::MultiSelect => f.write_str("MultiSelect"),
            RelationFormat::Date => f.write_str("Date"),
            RelationFormat::FileOrMedia => f.write_str("FileOrMedia"),
            RelationFormat::Checkbox => f.write_str("Checkbox"),
            RelationFormat::Url => f.write_str("Url"),
            RelationFormat::Email => f.write_str("Email"),
            RelationFormat::Phone => f.write_str("Phone"),
            RelationFormat::Object { types } => {
                f.write_str("Object { types: [")?;
                let last_index = types.len().saturating_sub(1);
                types.iter().enumerate().try_for_each(|(index, id)| {
                    id.fmt(f)?;

                    if last_index != index {
                        f.write_str(", ")?;
                    }

                    Ok(())
                })?;
                f.write_str("] }")?;

                Ok(())
            }
        }
    }
}

impl RelationFormat {
    fn from_internal(
        internal: InternalRelationFormat,
        object_types: Option<BTreeSet<ObjectTypeId>>,
    ) -> Self {
        match internal {
            InternalRelationFormat::Longtext => RelationFormat::Text,
            InternalRelationFormat::Shorttext => RelationFormat::Text,
            InternalRelationFormat::Number => RelationFormat::Number,
            InternalRelationFormat::Status => RelationFormat::Select,
            InternalRelationFormat::Tag => RelationFormat::MultiSelect,
            InternalRelationFormat::Date => RelationFormat::Date,
            InternalRelationFormat::File => RelationFormat::FileOrMedia,
            InternalRelationFormat::Checkbox => RelationFormat::Checkbox,
            InternalRelationFormat::Url => RelationFormat::Url,
            InternalRelationFormat::Email => RelationFormat::Email,
            InternalRelationFormat::Phone => RelationFormat::Phone,
            InternalRelationFormat::Emoji => {
                panic!("Creating `Emoji` formatted relations isn't supported by AnyType apps as of v0.39.0")
            }
            InternalRelationFormat::Object => RelationFormat::Object {
                types: object_types.unwrap_or_default(),
            },
            InternalRelationFormat::Relations => {
                panic!("Creating `Relations` formatted relation isn't supported by AnyType apps as of v0.39.0")
            }
        }
    }
}

impl From<&RelationFormat> for f64 {
    fn from(value: &RelationFormat) -> Self {
        let internal = match value {
            RelationFormat::Text => InternalRelationFormat::Longtext,
            RelationFormat::Number => InternalRelationFormat::Number,
            RelationFormat::Select => InternalRelationFormat::Status,
            RelationFormat::MultiSelect => InternalRelationFormat::Tag,
            RelationFormat::Date => InternalRelationFormat::Date,
            RelationFormat::FileOrMedia => InternalRelationFormat::File,
            RelationFormat::Checkbox => InternalRelationFormat::Checkbox,
            RelationFormat::Url => InternalRelationFormat::Url,
            RelationFormat::Email => InternalRelationFormat::Email,
            RelationFormat::Phone => InternalRelationFormat::Phone,
            RelationFormat::Object { .. } => InternalRelationFormat::Object,
        };

        i32::from(internal) as f64
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationId(ObjectId);

impl Display for RelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoProstValue for RelationId {
    fn into_prost(self) -> prost_types::Value {
        self.0.into_prost()
    }
}

impl TryFromProst for RelationId {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        ObjectId::try_from_prost(kind).map(RelationId)
    }
}

impl From<RelationId> for ObjectId {
    fn from(value: RelationId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationKey(pub(crate) String);

impl TryFromProst for RelationKey {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError>
    where
        Self: Sized,
    {
        String::try_from_prost(kind).map(RelationKey)
    }
}

#[derive(Debug)]
pub enum RelationValue {
    Text(String),
    Number(f64),
    // TODO:
    // Select
    // MultiSelect
    Date(NaiveDateTime),
    // FileOrMedia
    Checkbox(bool),
    Url(String),
    Email(String),
    Phone(String),
    Object(Vec<Object>),
}

impl RelationValue {
    pub fn format(&self) -> RelationFormat {
        match self {
            RelationValue::Text(_) => RelationFormat::Text,
            RelationValue::Number(_) => RelationFormat::Number,
            RelationValue::Date(_) => RelationFormat::Date,
            RelationValue::Checkbox(_) => RelationFormat::Checkbox,
            RelationValue::Url(_) => RelationFormat::Url,
            RelationValue::Email(_) => RelationFormat::Email,
            RelationValue::Phone(_) => RelationFormat::Phone,
            RelationValue::Object(objects) => RelationFormat::Object {
                types: objects
                    .clone()
                    .into_iter()
                    .map(|object| object.ty)
                    .collect(),
            },
        }
    }
}

impl IntoProstValue for RelationValue {
    fn into_prost(self) -> prost_types::Value {
        match self {
            RelationValue::Text(string)
            | RelationValue::Url(string)
            | RelationValue::Email(string)
            | RelationValue::Phone(string) => string.into_prost(),
            RelationValue::Number(number) => number.into_prost(),
            RelationValue::Date(datetime) => (datetime.and_utc().timestamp() as f64).into_prost(),
            RelationValue::Checkbox(boolean) => boolean.into_prost(),
            RelationValue::Object(objects) => objects
                .into_iter()
                .map(|object| object.id().into_prost())
                .collect::<Vec<_>>()
                .into_prost(),
        }
    }
}

pub struct IncompatibleRelationValue {
    expected: RelationFormat,
    received: RelationFormat,
}

impl Display for IncompatibleRelationValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expected format doesn't match received format:\nexpected:{}\nreceived:{}",
            self.expected, self.received
        )
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relation {
    id: RelationId,
    name: String,
    pub(crate) relation_key: RelationKey,
    format: RelationFormat,
}

impl Relation {
    pub fn id(&self) -> RelationId {
        self.id
    }

    pub fn into_id(self) -> RelationId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn format(&self) -> &RelationFormat {
        &self.format
    }

    pub fn into_spec(self) -> RelationSpec {
        RelationSpec {
            name: self.name,
            format: self.format,
        }
    }

    pub fn as_spec(&self) -> RelationSpec {
        RelationSpec {
            name: self.name.clone(),
            format: self.format.clone(),
        }
    }

    pub(crate) fn validate(
        &self,
        value: RelationValue,
    ) -> Result<RelationDetail, IncompatibleRelationValue> {
        let expected_format = self.format();
        let received_format = value.format();
        if expected_format.is_superset(&received_format) {
            Ok(RelationDetail {
                key: self.relation_key.clone(),
                value,
            })
        } else {
            Err(IncompatibleRelationValue {
                expected: expected_format.clone(),
                received: received_format,
            })
        }
    }
}

impl TryFromProst for Relation {
    type Input = prost_types::Struct;

    fn try_from_prost(input: Self::Input) -> Result<Self, ProstConversionError>
    where
        Self: Sized,
    {
        use crate::pb::models::object_type::Layout;

        let mut value = ProstStruct::from(input);

        let layout = value.take_enum::<Layout>("layout")?;
        assert!(layout == Layout::Relation);

        let id = value.take::<RelationId>("id")?;
        let name = value.take::<String>("name")?;
        let relation_key = value.take::<RelationKey>("relationKey")?;
        let format = value.take_enum::<InternalRelationFormat>("relationFormat")?;
        let object_types =
            value.take_optional::<BTreeSet<ObjectTypeId>>("relationFormatObjectTypes")?;

        Ok(Self {
            id,
            name,
            relation_key,
            format: RelationFormat::from_internal(format, object_types),
        })
    }
}

impl crate::space::SearchOutput for Relation {
    const LAYOUT: &'static [crate::pb::models::object_type::Layout] =
        &[crate::pb::models::object_type::Layout::Relation];
    type Id = RelationId;
}

/// Internal value only obtainable via [Relation::validate] that guarantees a relation's type has
/// been validated
pub(crate) struct RelationDetail {
    key: RelationKey,
    value: RelationValue,
}

impl RelationDetail {
    pub(crate) fn into_raw_parts(self) -> (String, prost_types::Value) {
        (self.key.0, self.value.into_prost())
    }
}
