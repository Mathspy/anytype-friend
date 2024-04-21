use crate::pb::{
    self, client_commands_client::ClientCommandsClient, models::block::content::dataview::Filter,
};
use crate::prost_ext::{IntoProstValue, TryFromProst};
use crate::request::RequestWithToken;

#[derive(Debug)]
pub struct Space {
    pub(crate) client: ClientCommandsClient<tonic::transport::Channel>,
    pub(crate) token: String,
    pub(crate) info: pb::models::account::Info,
}

/// Internal trait representing a known AnyType object layout.
///
/// This trait should only be implemented for types that should never fail their
/// TryFrom conversion
pub(crate) trait SearchOutput: TryFromProst<Input = prost_types::Struct> {
    const LAYOUT: pb::models::object_type::Layout;
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
                value: Some(vec![self.info.account_space_id.clone().into_prost()].into_prost()),

                ..Default::default()
            },
            // Always filter for only objects that match the desired output type
            Filter {
                operator: Operator::And.into(),
                relation_key: "layout".to_string(),
                condition: Condition::Equal.into(),
                value: Some((i32::from(O::LAYOUT) as f64).into_prost()),

                ..Default::default()
            },
        ]);

        let response = self
            .client
            .clone()
            .object_search(RequestWithToken {
                request: pb::rpc::object::search::Request {
                    filters,
                    ..Default::default()
                },
                token: &self.token,
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
            .map(O::try_from_prost)
            // TODO: We are guranteed via the trait SearchOutput that this
            // shouldn't need to filter anything, if it were to filter something
            // we should still warn though as that would imply bugs in the
            // internal code
            .filter_map(Result::ok)
            .collect::<Vec<_>>())
    }
}
