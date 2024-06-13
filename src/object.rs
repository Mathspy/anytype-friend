use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Debug, Display},
};

use chrono::DateTime;
use cid::CidGeneric;

use crate::{
    object_type::{ObjectType, ObjectTypeId, ObjectTypeUnresolved},
    prost_ext::{IntoProstValue, ProstConversionError, ProstStruct, TryFromProst},
    relation::{IncompatibleRelationValue, Relation, RelationFormat, RelationValue},
    space::Space,
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(CidGeneric<32>);

// Use's Display implementation for Debug.
// The Debug implementation is really unhelpful in our usecase.
impl Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl IntoProstValue for ObjectId {
    fn into_prost(self) -> prost_types::Value {
        format!("{}", self.0).into_prost()
    }
}

impl TryFromProst for ObjectId {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        let string = String::try_from_prost(kind)?;
        let cid = CidGeneric::<32>::try_from(string)
            .expect("ObjectIds are always valid CID with 32 bytes");
        Ok(ObjectId(cid))
    }
}

pub struct ObjectSpec {
    pub ty: ObjectType,
    pub name: String,
}

pub struct ObjectDescription {
    pub ty: ObjectType,
    pub name: String,
    pub relations: HashMap<Relation, RelationValue>,
}

impl ObjectSpec {
    pub fn as_description(&self) -> ObjectDescription {
        ObjectDescription {
            ty: self.ty.clone(),
            name: self.name.clone(),
            relations: HashMap::new(),
        }
    }
}

impl TryFrom<ObjectDescription> for prost_types::Struct {
    type Error = IncompatibleRelationValue;

    fn try_from(value: ObjectDescription) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), value.name.into_prost());
        fields.extend(
            value
                .relations
                .into_iter()
                // This is necessary because anytype-heart does not validate ANY of the relation
                // values sent to it. Yes if you set a relation format to Number and then send a
                // string it will just wholeheartedly accept it, it will even return it back to you
                // if you query for it later without any errors
                .map(|(relation, value)| {
                    relation
                        .validate(value)
                        .map(|detail| detail.into_raw_parts())
                })
                .collect::<Result<Vec<_>, Self::Error>>()?,
        );

        Ok(prost_types::Struct { fields })
    }
}

pub(crate) struct ObjectUnresolved {
    id: ObjectId,
    name: String,
    pub(crate) ty: ObjectTypeId,
    relations: prost_types::Struct,
}

impl ObjectUnresolved {
    pub fn resolve(self, space: Space) -> Object {
        Object {
            space,

            id: self.id,
            name: self.name,
            ty: self.ty,
            relations: self.relations,
        }
    }
}

impl crate::space::SearchOutput for ObjectUnresolved {
    const LAYOUT: &'static [crate::pb::models::object_type::Layout] = &[
        crate::pb::models::object_type::Layout::Basic,
        crate::pb::models::object_type::Layout::Bookmark,
    ];
    type Id = ObjectId;
}

impl TryFromProst for ObjectUnresolved {
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
        let ty = value.take::<ObjectTypeId>("type")?;

        Ok(Self {
            id,
            name,
            ty,
            relations: value.into_inner(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    space: Space,

    id: ObjectId,
    name: String,
    pub(crate) ty: ObjectTypeId,
    relations: prost_types::Struct,
}

impl Object {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // TODO: I don't think it's ideal this is async, we might want to resolve objects in a way that
    // allows us to pass their type with space
    pub async fn ty(&self) -> Result<ObjectType, tonic::Status> {
        self.space
            .get_objects::<ObjectTypeUnresolved>([self.ty])
            .await?
            .swap_remove(0)
            .slow_resolve(self.space.clone())
            .await
    }
}

impl Object {
    pub async fn get(&self, key: &Relation) -> Option<RelationValue> {
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
            RelationFormat::Date => f64::try_from_prost(kind)
                .map(|number| DateTime::from_timestamp(number as i64, 0).expect("unreachable"))
                .map(|datetime| datetime.naive_utc())
                .map(RelationValue::Date)
                .map(Some)
                .expect("unreachable"),
            RelationFormat::Checkbox => bool::try_from_prost(kind)
                .map(RelationValue::Checkbox)
                .map(Some)
                .expect("unreachable"),
            RelationFormat::Url => String::try_from_prost(kind)
                .map(RelationValue::Url)
                .map(Some)
                .expect("unreachable"),
            RelationFormat::Email => String::try_from_prost(kind)
                .map(RelationValue::Email)
                .map(Some)
                .expect("unreachable"),
            RelationFormat::Phone => String::try_from_prost(kind)
                .map(RelationValue::Phone)
                .map(Some)
                .expect("unreachable"),
            RelationFormat::Object { .. } => {
                let ids = <Vec<ObjectId>>::try_from_prost(kind).expect("unreachable");
                let objects = self
                    .space
                    .get_objects::<ObjectUnresolved>(ids)
                    .await
                    .expect("unreachable")
                    .into_iter()
                    .map(|object| object.resolve(self.space.clone()))
                    .collect::<Vec<_>>();

                Some(RelationValue::Object(objects))
            }
            _ => todo!(),
        }
    }

    pub async fn set(
        &self,
        key: &Relation,
        value: RelationValue,
    ) -> Result<Option<RelationValue>, tonic::Status> {
        let previous_value = self.get(key).await;

        self.space
            .set_relation(
                self.id,
                key.validate(value)
                    .map_err(|error| tonic::Status::failed_precondition(format!("{error}")))?,
            )
            .await?;

        Ok(previous_value)
    }
}
