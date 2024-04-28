use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
};

use crate::{
    object_type::ObjectType,
    prost_ext::{IntoProstValue, ProstConversionError, TryFromProst},
    relation::{Relation, RelationValue},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub(crate) String);

impl Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl IntoProstValue for ObjectId {
    fn into_prost(self) -> prost_types::Value {
        self.0.into_prost()
    }
}

impl TryFromProst for ObjectId {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        String::try_from_prost(kind).map(ObjectId)
    }
}

pub struct ObjectDescription {
    pub ty: ObjectType,
    pub name: String,
    pub relations: HashMap<Relation, RelationValue>,
}

impl From<ObjectDescription> for prost_types::Struct {
    fn from(value: ObjectDescription) -> Self {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), value.name.into_prost());
        fields.extend(
            value
                .relations
                .into_iter()
                .map(|(relation, value)| (relation.relation_key.0, value.into_prost())),
        );

        prost_types::Struct { fields }
    }
}
