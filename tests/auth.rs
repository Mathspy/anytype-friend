mod utils;

use anytype_friend::{AnytypeClient, NetworkSync};
use utils::run_with_service;

#[tokio::test]
async fn can_create_an_account_and_authenticate_with_it() {
    let temp_dir = tempdir::TempDir::new("anytype-friend").unwrap();
    let temp_dir_path = temp_dir.path();

    let (mnemonic, account_id) = run_with_service(|port| async move {
        let (mnemonic, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .create_account_with_path("Test Client", NetworkSync::LocalOnly, temp_dir_path)
            .await
            .unwrap();

        (mnemonic, client.get_account().id.clone())
    })
    .await;

    run_with_service(|port| async move {
        let client = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .authenticate_with_path(&mnemonic, temp_dir_path)
            .await
            .unwrap();

        assert_eq!(client.get_account().id, account_id);
    })
    .await;
}
