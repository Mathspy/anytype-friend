use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use crate::{
    object::ObjectId,
    prost_ext::{IntoProstValue, ProstConversionError, ProstStruct, TryFromProst},
    relation::{Relation, RelationId, RelationSpec},
    unique_key::UniqueKey,
};

pub struct ObjectTypeSpec {
    /// The name of the object type
    pub name: String,
    pub recommended_relations: BTreeSet<RelationSpec>,
}

impl ObjectTypeSpec {
    pub(crate) fn to_struct(&self, relations: Vec<RelationId>) -> prost_types::Struct {
        prost_types::Struct {
            fields: BTreeMap::from([
                ("name".to_string(), self.name.clone().into_prost()),
                (
                    "recommendedRelations".to_string(),
                    relations
                        .into_iter()
                        .map(|relation_id| relation_id.into_prost())
                        .collect::<Vec<_>>()
                        .into_prost(),
                ),
            ]),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectTypeId(ObjectId);

impl Display for ObjectTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoProstValue for ObjectTypeId {
    fn into_prost(self) -> prost_types::Value {
        self.0.into_prost()
    }
}

impl TryFromProst for ObjectTypeId {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        ObjectId::try_from_prost(kind).map(ObjectTypeId)
    }
}

impl From<ObjectTypeId> for ObjectId {
    fn from(value: ObjectTypeId) -> Self {
        value.0
    }
}

pub(crate) struct ObjectTypeUnresolved {
    id: ObjectTypeId,
    name: String,
    unique_key: UniqueKey,
    pub(crate) recommended_relations: BTreeSet<RelationId>,
}

impl TryFromProst for ObjectTypeUnresolved {
    type Input = prost_types::Struct;

    fn try_from_prost(input: Self::Input) -> Result<Self, ProstConversionError>
    where
        Self: Sized,
    {
        use crate::pb::models::object_type::Layout;

        let mut value = ProstStruct::from(input);

        let layout = value.take_enum::<Layout>("layout")?;
        assert!(layout == Layout::ObjectType);

        let id = value.take::<ObjectTypeId>("id")?;
        let name = value.take::<String>("name")?;
        let unique_key = value.take::<UniqueKey>("uniqueKey")?;
        let recommended_relations = value.take::<BTreeSet<RelationId>>("recommendedRelations")?;

        Ok(Self {
            id,
            name,
            unique_key,
            recommended_relations,
        })
    }
}

impl crate::space::SearchOutput for ObjectTypeUnresolved {
    const LAYOUT: &'static [crate::pb::models::object_type::Layout] =
        &[crate::pb::models::object_type::Layout::ObjectType];
    type Id = ObjectTypeId;
}

impl ObjectTypeUnresolved {
    pub fn resolve(self, recommended_relations: BTreeSet<Relation>) -> ObjectType {
        ObjectType {
            id: self.id,
            name: self.name,
            unique_key: self.unique_key,
            recommended_relations,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectType {
    id: ObjectTypeId,
    name: String,
    pub(crate) unique_key: UniqueKey,
    recommended_relations: BTreeSet<Relation>,
}

impl ObjectType {
    pub fn id(&self) -> ObjectTypeId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn recommended_relations(&self) -> &BTreeSet<Relation> {
        &self.recommended_relations
    }
}
