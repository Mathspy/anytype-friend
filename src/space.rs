use crate::pb::{self, client_commands_client::ClientCommandsClient};

#[derive(Debug)]
pub struct Space {
    pub(crate) client: ClientCommandsClient<tonic::transport::Channel>,
    pub(crate) token: String,
    pub(crate) info: pb::models::account::Info,
}
