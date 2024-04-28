use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
};

use crate::{
    object_type::ObjectType,
    prost_ext::{IntoProstValue, ProstConversionError, ProstStruct, TryFromProst},
    relation::{Relation, RelationValue},
    RelationFormat,
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

#[derive(Debug)]
pub struct Object {
    id: ObjectId,
    name: String,
    is_hidden: bool,
    relations: prost_types::Struct,
}

impl Object {
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl TryFromProst for Object {
    type Input = prost_types::Struct;

    fn try_from_prost(input: Self::Input) -> Result<Self, ProstConversionError>
    where
        Self: Sized,
    {
        use crate::pb::models::object_type::Layout;

        let mut value = ProstStruct::from(input);

        let layout = value.take_enum::<Layout>("layout")?;
        assert!(&[Layout::Basic, Layout::Bookmark].contains(&layout));

        let id = value.take::<ObjectId>("id")?;
        let name = value.take::<String>("name")?;
        let is_hidden = value.take_optional::<bool>("isHidden")?.unwrap_or_default();

        Ok(Self {
            id,
            name,
            is_hidden,
            relations: value.into_inner(),
        })
    }
}

impl Object {
    pub fn get(&self, key: &Relation) -> Option<RelationValue> {
        let kind = self
            .relations
            .fields
            .get(&key.relation_key.0)?
            .kind
            // TODO: This clone is a tiny bit sad but quite hard to avoid right now
            .clone()?;

        // The below expects SHOULD be unreachable because of how the rest of the public API
        // works the main error we expect to see here is IncorrectKind but because we pass
        // to this function a whole Relation and the only way to create a Relation is via
        // the Space APIs that will get the correct Relation and its format, we KNOW that if
        // that relation exists on some type it MUST have that format.
        //
        // The only way I can think of to cause this to trigger is by changing a Relation
        // format WHILE the code is running, which would be unfortunate but I accept that being broken for now
        match key.format() {
            RelationFormat::Text => String::try_from_prost(kind)
                .map(RelationValue::Text)
                .map(Some)
                .expect("unreachable"),
            RelationFormat::Number => f64::try_from_prost(kind)
                .map(RelationValue::Number)
                .map(Some)
                .expect("unreachable"),
            _ => todo!(),
        }
    }
}

impl crate::space::SearchOutput for Object {
    const LAYOUT: &'static [crate::pb::models::object_type::Layout] = &[
        crate::pb::models::object_type::Layout::Basic,
        crate::pb::models::object_type::Layout::Bookmark,
    ];

    fn is_hidden(&self) -> bool {
        self.is_hidden
    }
}
