use std::collections::BTreeSet;
use std::ops::Not;
use std::sync::Arc;

use futures_util::stream::FuturesUnordered;
use futures_util::TryStreamExt;

use crate::object::{Object, ObjectDescription, ObjectId, ObjectSpec, ObjectUnresolved};
use crate::object_type::{ObjectType, ObjectTypeSpec, ObjectTypeUnresolved};
use crate::pb::{
    self, client_commands_client::ClientCommandsClient, models::block::content::dataview::Filter,
};
use crate::prost_ext::{IntoProstValue, ProstStruct, TryFromProst};
use crate::relation::{Relation, RelationSpec};
use crate::request::RequestWithToken;

#[derive(Debug)]
pub(crate) struct SpaceInner {
    pub(crate) client: ClientCommandsClient<tonic::transport::Channel>,
    pub(crate) token: String,
    pub(crate) info: pb::models::account::Info,
}

#[derive(Debug, Clone)]
pub struct Space {
    pub(crate) inner: Arc<SpaceInner>,
}

/// Internal trait representing a known AnyType object layout.
///
/// This trait should only be implemented for types that should never fail their
/// TryFrom conversion
pub(crate) trait SearchOutput: TryFromProst<Input = prost_types::Struct> {
    const LAYOUT: &'static [pb::models::object_type::Layout];
    type Id: Into<ObjectId>;
}

impl Space {
    async fn search_objects<O>(&self, mut filters: Vec<Filter>) -> Result<Vec<O>, tonic::Status>
    where
        O: SearchOutput,
    {
        use pb::models::block::content::dataview::filter::{Condition, Operator};

        filters.extend([
            // Always filter for only objects in this space
            Filter {
                operator: Operator::And.into(),
                relation_key: "spaceId".to_string(),
                condition: Condition::In.into(),
                value: Some(
                    vec![self.inner.info.account_space_id.clone().into_prost()].into_prost(),
                ),

                ..Default::default()
            },
            // Always filter for only objects that match the desired output type
            Filter {
                operator: Operator::And.into(),
                relation_key: "layout".to_string(),
                condition: Condition::In.into(),
                value: Some(
                    O::LAYOUT
                        .iter()
                        .map(|layout| (i32::from(*layout) as f64).into_prost())
                        .collect::<Vec<_>>()
                        .into_prost(),
                ),

                ..Default::default()
            },
        ]);

        let response = self
            .inner
            .client
            .clone()
            .object_search(RequestWithToken {
                request: pb::rpc::object::search::Request {
                    filters,
                    ..Default::default()
                },
                token: &self.inner.token,
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::object::search::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
            }
        }

        Ok(response
            .records
            .into_iter()
            .map(ProstStruct::from)
            // We always filter outputs that are hidden so that they aren't used
            // by mistake anywhere else
            .filter_map(|mut fields| {
                fields
                    .take_optional::<bool>("isHidden")
                    .expect("isHidden field is always a boolean")
                    .unwrap_or_default()
                    .not()
                    .then_some(fields.into_inner())
            })
            .map(O::try_from_prost)
            // TODO: We are guranteed via the trait SearchOutput that this
            // shouldn't need to filter anything, if it were to filter something
            // we should still warn though as that would imply bugs in the
            // internal code
            .filter_map(Result::ok)
            .collect::<Vec<_>>())
    }

    pub(crate) async fn get_objects<O>(
        &self,
        ids: impl IntoIterator<Item = O::Id>,
    ) -> Result<Vec<O>, tonic::Status>
    where
        O: SearchOutput,
    {
        use pb::models::block::content::dataview::filter::{Condition, Operator};

        let objects = self
            .search_objects::<O>(vec![Filter {
                operator: Operator::And.into(),
                relation_key: "id".to_string(),
                condition: Condition::In.into(),
                value: Some(
                    ids.into_iter()
                        .map(|id| id.into().into_prost())
                        .collect::<Vec<_>>()
                        .into_prost(),
                ),

                ..Default::default()
            }])
            .await?;

        Ok(objects)
    }

    pub async fn get_relation(
        &self,
        relation_spec: &RelationSpec,
    ) -> Result<Option<Relation>, tonic::Status> {
        use pb::models::block::content::dataview::filter::{Condition, Operator};

        let mut relations = self
            .search_objects::<Relation>(vec![Filter {
                operator: Operator::And.into(),
                relation_key: "name".to_string(),
                condition: Condition::Equal.into(),
                value: Some(relation_spec.name.clone().into_prost()),

                ..Default::default()
            }])
            .await?;

        match relations.len() {
            0 => Ok(None),
            1 => {
                let relation = relations.swap_remove(0);

                if *relation.format() == relation_spec.format {
                    Ok(Some(relation))
                } else {
                    Err(tonic::Status::failed_precondition(format!(
                        "Relation `{}` exists but has a different format {} from requested format {}",
                        relation.name(),
                        relation.format(),
                        relation_spec.format
                    )))
                }
            }
            _ => Err(tonic::Status::failed_precondition(format!(
                "More than one relation with same name {}",
                relation_spec.name
            ))),
        }
    }

    pub async fn create_relation(
        &self,
        relation_spec: &RelationSpec,
    ) -> Result<Relation, tonic::Status> {
        let response = self
            .inner
            .client
            .clone()
            .object_create_relation(RequestWithToken {
                request: pb::rpc::object::create_relation::Request {
                    space_id: self.inner.info.account_space_id.clone(),
                    details: Some(relation_spec.clone().into()),
                },
                token: &self.inner.token,
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::object::create_relation::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
            }
        }

        let Some(details) = response.details else {
            return Err(tonic::Status::internal(
                "anytype-heart did not respond with a relation's details",
            ));
        };

        Relation::try_from_prost(details)
            .map_err(|error| tonic::Status::internal(format!("{error}")))
    }

    pub async fn obtain_relation(
        &self,
        relation_spec: &RelationSpec,
    ) -> Result<Relation, tonic::Status> {
        match self.get_relation(relation_spec).await? {
            None => self.create_relation(relation_spec).await,
            Some(relation) => Ok(relation),
        }
    }

    pub async fn get_object_type(
        &self,
        object_type_spec: &ObjectTypeSpec,
    ) -> Result<Option<ObjectType>, tonic::Status> {
        use pb::models::block::content::dataview::filter::{Condition, Operator};

        let mut object_types = self
            .search_objects::<ObjectTypeUnresolved>(vec![Filter {
                operator: Operator::And.into(),
                relation_key: "name".to_string(),
                condition: Condition::Like.into(),
                value: Some(object_type_spec.name.clone().into_prost()),

                ..Default::default()
            }])
            .await?;

        match object_types.len() {
            0 => Ok(None),
            1 => {
                let object_type = object_types.swap_remove(0);

                let output = object_type.slow_resolve(self.clone()).await?;
                let relations_specs = output
                    .recommended_relations()
                    .iter()
                    .map(Relation::as_spec)
                    .collect::<BTreeSet<_>>();

                if relations_specs == object_type_spec.recommended_relations {
                    Ok(Some(output))
                } else {
                    Err(tonic::Status::failed_precondition(format!(
                        "ObjectType `{}` exists but has different recommended relations from requested recommended relations:
Requested recommended relations: {:?}

Received recommended relations: {:?}",
                        object_type_spec.name,
                        relations_specs,
                        object_type_spec.recommended_relations
                    )))
                }
            }
            _ => Err(tonic::Status::failed_precondition(format!(
                "More than one object type with same name {}",
                object_type_spec.name
            ))),
        }
    }

    pub async fn create_object_type(
        &self,
        object_type_spec: &ObjectTypeSpec,
    ) -> Result<ObjectType, tonic::Status> {
        let recommended_relations = object_type_spec
            .recommended_relations
            .iter()
            .map(|relation_spec| async { self.obtain_relation(relation_spec).await })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<BTreeSet<_>>()
            .await?;
        let relation_ids = recommended_relations
            .clone()
            .into_iter()
            .map(|relation| relation.into_id())
            .collect::<Vec<_>>();

        let response = self
            .inner
            .client
            .clone()
            .object_create_object_type(RequestWithToken {
                request: pb::rpc::object::create_object_type::Request {
                    space_id: self.inner.info.account_space_id.clone(),
                    details: Some(object_type_spec.to_struct(relation_ids)),

                    ..Default::default()
                },
                token: &self.inner.token,
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::object::create_object_type::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
            }
        }

        let Some(details) = response.details else {
            return Err(tonic::Status::internal(
                "anytype-heart did not respond with a object type's details",
            ));
        };

        ObjectTypeUnresolved::try_from_prost(details)
            .map(|object_type| object_type.resolve(recommended_relations))
            .map_err(|error| tonic::Status::internal(format!("{error}")))
    }

    pub async fn obtain_object_type(
        &self,
        object_type_spec: &ObjectTypeSpec,
    ) -> Result<ObjectType, tonic::Status> {
        match self.get_object_type(object_type_spec).await? {
            None => self.create_object_type(object_type_spec).await,
            Some(object_type) => Ok(object_type),
        }
    }

    pub async fn get_object(
        &self,
        object_spec: &ObjectSpec,
    ) -> Result<Option<Object>, tonic::Status> {
        use pb::models::block::content::dataview::filter::{Condition, Operator};

        let mut objects = self
            .search_objects::<ObjectUnresolved>(vec![
                Filter {
                    operator: Operator::And.into(),
                    relation_key: "name".to_string(),
                    condition: Condition::Like.into(),
                    value: Some(object_spec.name.clone().into_prost()),

                    ..Default::default()
                },
                Filter {
                    operator: Operator::And.into(),
                    relation_key: "type".to_string(),
                    condition: Condition::Equal.into(),
                    value: Some(object_spec.ty.id().into_prost()),

                    ..Default::default()
                },
            ])
            .await?;

        // This function is a bit unique compared to other gets in that it won't error if an object
        // exist with the same name but a different type. It feels like there are a lot of usecases
        // for having different objects with the same name but different types. Kind of like why
        // shadowing comes in so handy in Rust.
        match objects.len() {
            0 => Ok(None),
            1 => Ok(Some(objects.swap_remove(0).resolve(self.clone()))),
            _ => Err(tonic::Status::failed_precondition(format!(
                "More than one object with same name {}",
                object_spec.name
            ))),
        }
    }

    pub async fn create_object(&self, object: ObjectDescription) -> Result<Object, tonic::Status> {
        let response =
            self.inner
                .client
                .clone()
                .object_create(RequestWithToken {
                    request: pb::rpc::object::create::Request {
                        space_id: self.inner.info.account_space_id.clone(),
                        object_type_unique_key: object.ty.unique_key.clone().0,
                        details: Some(object.try_into().map_err(|error| {
                            tonic::Status::failed_precondition(format!("{error}"))
                        })?),

                        ..Default::default()
                    },
                    token: &self.inner.token,
                })
                .await?
                .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::object::create::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
            }
        }

        let Some(details) = response.details else {
            return Err(tonic::Status::internal(
                "anytype-heart did not respond with a object's details",
            ));
        };

        ObjectUnresolved::try_from_prost(details)
            .map(|object| object.resolve(self.clone()))
            .map_err(|error| tonic::Status::internal(format!("{error}")))
    }

    pub async fn obtain_object(&self, object_spec: &ObjectSpec) -> Result<Object, tonic::Status> {
        match self.get_object(object_spec).await? {
            None => self.create_object(object_spec.as_description()).await,
            Some(object) => Ok(object),
        }
    }
}
