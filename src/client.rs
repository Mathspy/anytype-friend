use std::path::{Path, PathBuf};

use crate::pb::{self, client_commands_client::ClientCommandsClient, models::Account};
use crate::request::RequestWithToken;
use crate::space::Space;

pub struct AnytypeClient {
    inner: ClientCommandsClient<tonic::transport::Channel>,
    disable_local_network_sync: bool,
    network_mode: i32,
    root_path: Option<PathBuf>,
}

pub enum NetworkSync {
    Sync,
    LocalOnly,
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
        use pb::rpc::account::NetworkMode;
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

        Ok(Self {
            inner: client,
            disable_local_network_sync: false,
            network_mode: NetworkMode::DefaultConfig.into(),
            root_path: None,
        })
    }

    fn calculate_root_path(&self) -> Result<PathBuf, tonic::Status> {
        if let Some(root_path) = &self.root_path {
            return Ok(root_path.clone());
        }

        let Some(home_dir) = dirs::home_dir() else {
            return Err(tonic::Status::failed_precondition("Missing home directory"));
        };

        Ok(home_dir.join(MACOS_PATH))
    }

    pub async fn authenticate(
        mut self,
        mnemonic: &str,
    ) -> Result<AuthorizedAnytypeClient, tonic::Status> {
        let root_path = self
            .calculate_root_path()?
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

        let token = self.create_wallet_session(mnemonic).await?;

        let (mut event_listener, event_listener_task) = self.start_event_listener(&token);

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

        self.set_metrics().await?;

        let response = self
            .inner
            .account_select(pb::rpc::account::select::Request {
                id: account_id,
                root_path,
                disable_local_network_sync: self.disable_local_network_sync,
                network_mode: self.network_mode,
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

        Ok(AuthorizedAnytypeClient {
            inner: self.inner,
            token,
            account: Self::account_or_error(response.account)?,
            event_listener,
            event_listener_task,
        })
    }

    async fn create_wallet_session(&self, mnemonic: &str) -> Result<String, tonic::Status> {
        let response = self
            .inner
            .clone()
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

        Ok(response.token)
    }

    fn start_event_listener(
        &self,
        token: &str,
    ) -> (
        tokio::sync::mpsc::Receiver<pb::event::message::Value>,
        tokio::task::JoinHandle<()>,
    ) {
        let (event_emitter, event_listener) = tokio::sync::mpsc::channel(64);
        let event_listener_task = tokio::spawn({
            let client = self.inner.clone();
            let token = token.to_string();

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
                                        event_emitter
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
                        Ok(None) => {
                            tokio::task::yield_now().await;
                        }
                        Err(error) => {
                            // TODO: Anything we need to do here?
                            dbg!(error);
                            tokio::task::yield_now().await;
                        }
                    }
                }
            }
        });

        (event_listener, event_listener_task)
    }

    pub async fn create_account(
        mut self,
        name: &str,
    ) -> Result<(String, AuthorizedAnytypeClient), tonic::Status> {
        let root_path = self
            .calculate_root_path()?
            .into_os_string()
            .into_string()
            .expect("non utf-8 path root_path");

        let response = self
            .inner
            .wallet_create(pb::rpc::wallet::create::Request {
                root_path: root_path.clone(),
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::wallet::create::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
                Code::FailedToCreateLocalRepo => {
                    return Err(tonic::Status::internal(error.description))
                }
            }
        }

        let mnemonic = response.mnemonic;
        let token = self.create_wallet_session(&mnemonic).await?;

        let (event_listener, event_listener_task) = self.start_event_listener(&token);

        self.set_metrics().await?;

        let response = self
            .inner
            .account_create(pb::rpc::account::create::Request {
                name: name.to_string(),
                store_path: root_path,
                icon: 0,
                disable_local_network_sync: self.disable_local_network_sync,
                network_mode: self.network_mode,
                network_custom_config_file_path: String::new(),
                prefer_yamux_transport: false,
                avatar: None,
            })
            .await?
            .into_inner();

        if let Some(error) = response.error {
            use pb::rpc::account::create::response::error::Code;
            match error.code() {
                Code::Null => {}
                Code::UnknownError => return Err(tonic::Status::unknown(error.description)),
                Code::BadInput => return Err(tonic::Status::invalid_argument(error.description)),
                _ => todo!("So many unique error types..."),
            }
        }

        Ok((
            mnemonic,
            AuthorizedAnytypeClient {
                inner: self.inner,
                token,
                account: Self::account_or_error(response.account)?,
                event_listener,
                event_listener_task,
            },
        ))
    }

    pub fn with_root_path<P: AsRef<Path>>(self, path: P) -> Self {
        Self {
            root_path: Some(path.as_ref().to_path_buf()),
            ..self
        }
    }

    pub fn with_network_sync(self, network_sync: NetworkSync) -> Self {
        use pb::rpc::account::NetworkMode;

        let disable_local_network_sync = match &network_sync {
            NetworkSync::Sync => false,
            NetworkSync::LocalOnly => true,
        };

        let network_mode = match &network_sync {
            NetworkSync::Sync => NetworkMode::DefaultConfig,
            NetworkSync::LocalOnly => NetworkMode::LocalOnly,
        };

        Self {
            disable_local_network_sync,
            network_mode: network_mode.into(),
            ..self
        }
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

    async fn set_metrics(&self) -> Result<(), tonic::Status> {
        let response = self
            .inner
            .clone()
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

        Ok(())
    }

    fn account_or_error(account: Option<Account>) -> Result<Account, tonic::Status> {
        let Some(account) = account else {
            return Err(tonic::Status::internal(
                "anytype-heart did not respond with an account",
            ));
        };

        Ok(account)
    }
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

impl Drop for AuthorizedAnytypeClient {
    fn drop(&mut self) {
        self.event_listener_task.abort();
    }
}
