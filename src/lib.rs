mod pb {
    mod models {
        tonic::include_proto!("anytype.model");
    }

    tonic::include_proto!("anytype");
}

use pb::client_commands_client::ClientCommandsClient;

pub struct AnytypeClient {
    inner: ClientCommandsClient<tonic::transport::Channel>,
}

impl AnytypeClient {
    pub async fn connect(url: &str) -> Result<Self, tonic::transport::Error> {
        use std::str::FromStr;

        let url = tonic::transport::Endpoint::from_str(url)?;
        Ok(Self {
            inner: ClientCommandsClient::connect(url).await?,
        })
    }
}
