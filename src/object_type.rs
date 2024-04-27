use std::{collections::BTreeSet, fmt::Display};

use crate::{
    prost_ext::{IntoProstValue, ProstConversionError, ProstStruct, TryFromProst},
    relation::{RelationId, RelationSpec},
};

pub struct ObjectTypeSpec {
    /// The name of the object type
    pub name: String,
    pub relations: BTreeSet<RelationSpec>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectTypeId(String);

impl Display for ObjectTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
        String::try_from_prost(kind).map(ObjectTypeId)
    }
}

#[derive(Debug)]
pub struct ObjectType {
    id: ObjectTypeId,
    name: String,
    is_hidden: bool,
    pub(crate) recommended_relations: BTreeSet<RelationId>,
}

impl ObjectType {
    pub fn id(&self) -> &ObjectTypeId {
        &self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl TryFromProst for ObjectType {
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
        let is_hidden = value.take_optional::<bool>("isHidden")?.unwrap_or_default();
        let recommended_relations = value.take::<BTreeSet<RelationId>>("recommendedRelations")?;

        Ok(Self {
            id,
            name,
            is_hidden,
            recommended_relations,
        })
    }
}

impl crate::space::SearchOutput for ObjectType {
    const LAYOUT: crate::pb::models::object_type::Layout =
        crate::pb::models::object_type::Layout::ObjectType;

    fn is_hidden(&self) -> bool {
        self.is_hidden
    }
}
