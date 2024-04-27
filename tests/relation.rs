mod utils;

use anytype_friend::{AnytypeClient, NetworkSync, RelationFormat, RelationSpec};
use utils::run_with_service;

#[tokio::test]
async fn relation_can_obtain_a_preexisting_one() {
    let temp_dir = tempdir::TempDir::new("anytype-friend").unwrap();
    let temp_dir_path = temp_dir.path();

    run_with_service(|port| async move {
        let (_, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::NoSync)
            .with_root_path(temp_dir_path)
            .create_account("Test Client")
            .await
            .unwrap();

        let space = client.default_space().await.unwrap().unwrap();
        let spec = RelationSpec {
            name: "Due date".to_string(),
            format: RelationFormat::Date,
        };
        let relation = match space.get_relation(&spec).await.unwrap() {
            Some(relation) => relation,
            None => panic!("Due date relation doesn't exist on a new space"),
        };
        assert_eq!(relation.name(), "Due date");
        assert_eq!(*relation.format(), RelationFormat::Date);

        let obtained_relation = space.obtain_relation(&spec).await.unwrap();
        assert_eq!(relation.id(), obtained_relation.id());
    })
    .await;
}

#[tokio::test]
async fn relation_fails_to_obtain_on_mismatched_format() {
    let temp_dir = tempdir::TempDir::new("anytype-friend").unwrap();
    let temp_dir_path = temp_dir.path();

    run_with_service(|port| async move {
        let (_, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::NoSync)
            .with_root_path(temp_dir_path)
            .create_account("Test Client")
            .await
            .unwrap();

        let result = client
            .default_space()
            .await
            .unwrap()
            .unwrap()
            .obtain_relation(&RelationSpec {
                name: "Due date".to_string(),
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
async fn relation_can_obtain_a_new_one() {
    let temp_dir = tempdir::TempDir::new("anytype-friend").unwrap();
    let temp_dir_path = temp_dir.path();

    run_with_service(|port| async move {
        let (_, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::NoSync)
            .with_root_path(temp_dir_path)
            .create_account("Test Client")
            .await
            .unwrap();

        let space = client.default_space().await.unwrap().unwrap();
        let spec = RelationSpec {
            name: "Longitude".to_string(),
            format: RelationFormat::Number,
        };
        if space.get_relation(&spec).await.unwrap().is_some() {
            unreachable!("Longtiude is now a default anytype relation");
        }

        let relation = space.obtain_relation(&spec).await.unwrap();
        assert_eq!(relation.name(), "Longitude");
        assert_eq!(*relation.format(), RelationFormat::Number);
    })
    .await;
}
