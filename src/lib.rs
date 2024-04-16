mod request;
mod pb {
    pub(crate) mod models {
        tonic::include_proto!("anytype.model");
    }

    tonic::include_proto!("anytype");
}

use pb::{client_commands_client::ClientCommandsClient, models::Account};
use request::RequestWithToken;

pub struct AnytypeClient {
    inner: ClientCommandsClient<tonic::transport::Channel>,
}

pub struct AuthorizedAnytypeClient {
    inner: ClientCommandsClient<tonic::transport::Channel>,
    token: String,
    account: Account,
    event_listener: tokio::sync::mpsc::Receiver<pb::event::message::Value>,
    event_listener_task: tokio::task::JoinHandle<()>,
}

const MACOS_PATH: &str = "Library/Application Support/anytype/";

impl AnytypeClient {
    pub async fn connect(url: &str) -> Result<Self, tonic::transport::Error> {
        use std::str::FromStr;

        let url = tonic::transport::Endpoint::from_str(url)?;
        let mut client = ClientCommandsClient::connect(url).await?;

        let response = client
            .app_get_version(pb::rpc::app::get_version::Request {})
            .await
            .expect("app_get_version request to succeed")
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::app::get_version::response::error::Code;

            if error.code() != Code::Null {
                panic!(
                    "Failed to get anytype-heart server details: {}",
                    error.description
                );
            }
        };

        assert!(
            response.version == "v0.32.1",
            "anytype-friend currently only supports anytype-heart v0.32.1"
        );
        assert!(
            response.details == "build on 2024-03-05 15:37:42 +0000 UTC at #a7986fffadcc2031b1eb3372265db5dda05f4c6d",
            "anytype-friend currently only supports anytype-heart build a7986fffadcc2031b1eb3372265db5dda05f4c6d"
        );

        Ok(Self { inner: client })
    }

    pub async fn auth(mut self, mnemonic: &str) -> Result<AuthorizedAnytypeClient, tonic::Status> {
        let Some(home_dir) = dirs::home_dir() else {
            return Err(tonic::Status::failed_precondition("Missing home directory"));
        };

        let root_path = home_dir
            .join(MACOS_PATH)
            .into_os_string()
            .into_string()
            .expect("non utf-8 path root_path");

        let response = self
            .inner
            .wallet_recover(pb::rpc::wallet::recover::Request {
                root_path: root_path.clone(),
                mnemonic: mnemonic.to_string(),
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::wallet::recover::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
                Code::FailedToCreateLocalRepo => {
                    return Err(tonic::Status::internal(error.description))
                }
            }
        };

        let response = self
            .inner
            .wallet_create_session(pb::rpc::wallet::create_session::Request {
                auth: Some(pb::rpc::wallet::create_session::request::Auth::Mnemonic(
                    mnemonic.to_string(),
                )),
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::wallet::create_session::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
            }
        }

        let token = response.token;

        let (account_tx, mut event_listener) = tokio::sync::mpsc::channel(64);
        let event_listener_task = tokio::spawn({
            let client = self.inner.clone();
            let token = token.clone();

            async move {
                let response = client
                    .clone()
                    .listen_session_events(pb::StreamRequest { token })
                    .await
                    .unwrap();

                let mut stream = response.into_inner();

                loop {
                    match stream.message().await {
                        Ok(Some(event)) => {
                            for message in event.messages {
                                use pb::event::message::Value;

                                let Some(value) = message.value else {
                                    continue;
                                };

                                match &value {
                                    Value::AccountShow(_) => {
                                        account_tx
                                            .send(value)
                                            .await
                                            .expect("Event receiver dropped");
                                    }
                                    message => {
                                        // TODO: Properly log other messages in debug logs
                                        dbg!(message);
                                    }
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            // TODO: Anything we need to do here?
                            dbg!(error);
                        }
                    }
                }
            }
        });

        let response = self
            .inner
            .account_recover(RequestWithToken {
                request: pb::rpc::account::recover::Request {},
                token: &token,
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::account::recover::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
                Code::NeedToRecoverWalletFirst => {
                    return Err(tonic::Status::failed_precondition(error.description))
                }
            }
        }

        let Some(account_id) = Self::wait_account_id_event(&mut event_listener).await else {
            return Err(tonic::Status::internal(
                "AnytypeClient internal event queue was unexpectedly closed",
            ));
        };

        let response = self
            .inner
            .metrics_set_parameters(pb::rpc::metrics::set_parameters::Request {
                platform: "Mac".to_string(),
                version: "0.39.0".to_string(),
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::metrics::set_parameters::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
            }
        }

        let response = self
            .inner
            .account_select(pb::rpc::account::select::Request {
                id: account_id,
                root_path,
                disable_local_network_sync: false,
                network_mode: pb::rpc::account::NetworkMode::DefaultConfig.into(),
                network_custom_config_file_path: "".to_string(),
                prefer_yamux_transport: false,
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::account::select::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
                _ => todo!("So many unique error types..."),
            }
        }

        let Some(account) = response.account else {
            return Err(tonic::Status::internal(
                "anytype-heart did not respond with an account",
            ));
        };

        Ok(AuthorizedAnytypeClient {
            inner: self.inner,
            token,
            account,
            event_listener,
            event_listener_task,
        })
    }

    async fn wait_account_id_event(
        event_listener: &mut tokio::sync::mpsc::Receiver<pb::event::message::Value>,
    ) -> Option<String> {
        use pb::event::message::Value;

        while let Some(event) = event_listener.recv().await {
            match event {
                Value::AccountShow(show) => {
                    let Some(account) = show.account else {
                        continue;
                    };

                    return Some(account.id);
                }
                _ => {
                    unreachable!(
                        "event_listener should currently never recieve any events besides AccountShow"
                    );
                }
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct Space {
    client: ClientCommandsClient<tonic::transport::Channel>,
    token: String,
    info: pb::models::account::Info,
}

impl AuthorizedAnytypeClient {
    pub fn get_account(&self) -> &Account {
        &self.account
    }

    pub async fn default_space(&self) -> Result<Option<Space>, tonic::Status> {
        let Some(info) = self.account.info.as_ref() else {
            return Ok(None);
        };

        self.open_space(&info.account_space_id).await
    }

    pub async fn open_space(&self, space_id: &str) -> Result<Option<Space>, tonic::Status> {
        let response = self
            .inner
            .clone()
            .workspace_open(RequestWithToken {
                request: pb::rpc::workspace::open::Request {
                    space_id: space_id.to_string(),
                },
                token: &self.token,
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::workspace::open::response::error::Code;
            match error.code() {
                Code::Null => {}
                // TODO: This hack will hopefully not last forever, currently anytype-heart doesn't
                // really give any better way of detecting an incorrect space_id error though
                Code::UnknownError
                    if error.description
                        == "failed to get derived ids: failed to get space: space not exists" =>
                {
                    return Ok(None)
                }
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
            }
        };

        let Some(info) = response.info else {
            return Err(tonic::Status::internal(
                "anytype-heart did not respond with a space's info",
            ));
        };

        Ok(Some(Space {
            client: self.inner.clone(),
            token: self.token.clone(),
            info,
        }))
    }
}
