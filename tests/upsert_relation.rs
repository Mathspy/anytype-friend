mod utils;

use anytype_friend::{AnytypeClient, NetworkSync, RelationFormat, RelationSpec};
use utils::run_with_service;

#[tokio::test]
async fn upsert_relation_can_upsert_a_preexisting_one() {
    let temp_dir = tempdir::TempDir::new("anytype-friend").unwrap();
    let temp_dir_path = temp_dir.path();

    run_with_service(|port| async move {
        let (_, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::LocalOnly)
            .with_root_path(temp_dir_path)
            .create_account("Test Client")
            .await
            .unwrap();

        let relation = client
            .default_space()
            .await
            .unwrap()
            .unwrap()
            .upsert_relation(RelationSpec {
                name: "due date".to_string(),
                format: RelationFormat::Date,
            })
            .await
            .unwrap();

        // This also verifies that name is case insensitive
        assert_eq!(relation.get_name(), "Due date");
        assert_eq!(*relation.get_format(), RelationFormat::Date);
    })
    .await;
}

#[tokio::test]
async fn upsert_relation_fails_to_upsert_on_mismatched_format() {
    let temp_dir = tempdir::TempDir::new("anytype-friend").unwrap();
    let temp_dir_path = temp_dir.path();

    run_with_service(|port| async move {
        let (_, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::LocalOnly)
            .with_root_path(temp_dir_path)
            .create_account("Test Client")
            .await
            .unwrap();

        let result = client
            .default_space()
            .await
            .unwrap()
            .unwrap()
            .upsert_relation(RelationSpec {
                name: "due date".to_string(),
                format: RelationFormat::Text,
            })
            .await
            .unwrap_err();

        assert_eq!(
            result.message(),
            "Relation `Due date` exists but has a different format Date from requested format Text"
        );
    })
    .await;
}

#[tokio::test]
async fn upsert_relation_can_upsert_a_new_one() {
    let temp_dir = tempdir::TempDir::new("anytype-friend").unwrap();
    let temp_dir_path = temp_dir.path();

    run_with_service(|port| async move {
        let (_, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::LocalOnly)
            .with_root_path(temp_dir_path)
            .create_account("Test Client")
            .await
            .unwrap();

        let relation = client
            .default_space()
            .await
            .unwrap()
            .unwrap()
            .upsert_relation(RelationSpec {
                name: "Longitude".to_string(),
                format: RelationFormat::Number,
            })
            .await
            .unwrap();

        assert_eq!(relation.get_name(), "Longitude");
        assert_eq!(*relation.get_format(), RelationFormat::Number);
    })
    .await;
}
