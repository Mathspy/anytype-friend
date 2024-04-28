use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use chrono::NaiveDateTime;

use crate::{
    object::ObjectId,
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
        prost_types::Struct {
            fields: BTreeMap::from([
                ("name".to_string(), value.name.to_string().into_prost()),
                (
                    "relationFormat".to_string(),
                    f64::from(value.format).into_prost(),
                ),
            ]),
        }
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
                let last_index = types.len() - 1;
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

impl From<RelationFormat> for f64 {
    fn from(value: RelationFormat) -> Self {
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationId(String);

impl Display for RelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
        String::try_from_prost(kind).map(RelationId)
    }
}

impl From<RelationId> for ObjectId {
    fn from(value: RelationId) -> Self {
        Self(value.0)
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

#[derive(Debug, PartialEq)]
pub enum RelationValue {
    Text(String),
    Number(f64),
    // TODO:
    // Select
    // MultiSelect
    Date(NaiveDateTime),
    // FileOrMedia
    // Checkbox
    // Url
    // Email
    // Phone
    // Object
}

impl IntoProstValue for RelationValue {
    fn into_prost(self) -> prost_types::Value {
        match self {
            RelationValue::Text(string) => string.into_prost(),
            RelationValue::Number(number) => number.into_prost(),
            RelationValue::Date(datetime) => (datetime.and_utc().timestamp() as f64).into_prost(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relation {
    id: RelationId,
    name: String,
    is_hidden: bool,
    pub(crate) relation_key: RelationKey,
    format: RelationFormat,
}

impl Relation {
    pub fn id(&self) -> &RelationId {
        &self.id
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
        let is_hidden = value.take_optional::<bool>("isHidden")?.unwrap_or_default();
        let relation_key = value.take::<RelationKey>("relationKey")?;
        let format = value.take_enum::<InternalRelationFormat>("relationFormat")?;
        let object_types =
            value.take_optional::<BTreeSet<ObjectTypeId>>("relationFormatObjectTypes")?;

        Ok(Self {
            id,
            name,
            is_hidden,
            relation_key,
            format: RelationFormat::from_internal(format, object_types),
        })
    }
}

impl crate::space::SearchOutput for Relation {
    const LAYOUT: &'static [crate::pb::models::object_type::Layout] =
        &[crate::pb::models::object_type::Layout::Relation];

    fn is_hidden(&self) -> bool {
        self.is_hidden
    }
}
