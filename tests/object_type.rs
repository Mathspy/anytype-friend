mod utils;

use std::collections::BTreeSet;

use anytype_friend::{AnytypeClient, NetworkSync, ObjectTypeSpec, RelationFormat, RelationSpec};
use utils::run_with_service;

#[tokio::test]
async fn object_type_can_obtain_a_preexisting_one_without_relations() {
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

        let space = client.default_space().await.unwrap().unwrap();
        let spec = ObjectTypeSpec {
            name: "Bookmark".to_string(),
            relations: BTreeSet::from([
                RelationSpec {
                    name: "Tag".to_string(),
                    format: RelationFormat::MultiSelect,
                },
                RelationSpec {
                    name: "Description".to_string(),
                    format: RelationFormat::Text,
                },
                RelationSpec {
                    name: "Source".to_string(),
                    format: RelationFormat::Url,
                },
            ]),
        };

        let object_type = match space.get_object_type(&spec).await.unwrap() {
            Some(object_type) => object_type,
            None => panic!("Bookmark object type doesn't exist on a new space"),
        };
        assert_eq!(object_type.get_name(), "Bookmark");

        let obtained_object_type = space.obtain_object_type(&spec).await.unwrap();
        assert_eq!(object_type.id(), obtained_object_type.id());
    })
    .await;
}

#[tokio::test]
async fn object_type_can_obtain_a_preexisting_one_with_relations() {
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

        let space = client.default_space().await.unwrap().unwrap();
        let tag_relation = space
            .obtain_relation(&RelationSpec {
                name: "Tag".to_string(),
                format: RelationFormat::MultiSelect,
            })
            .await
            .unwrap();

        let description_relation = space
            .obtain_relation(&RelationSpec {
                name: "Description".to_string(),
                format: RelationFormat::Text,
            })
            .await
            .unwrap();

        let source_relation = space
            .obtain_relation(&RelationSpec {
                name: "Source".to_string(),
                format: RelationFormat::Url,
            })
            .await
            .unwrap();

        let spec = ObjectTypeSpec {
            name: "Bookmark".to_string(),
            relations: BTreeSet::from([
                tag_relation.into_spec(),
                description_relation.into_spec(),
                source_relation.into_spec(),
            ]),
        };

        let object_type = match space.get_object_type(&spec).await.unwrap() {
            Some(object_type) => object_type,
            None => panic!("Bookmark object type doesn't exist on a new space"),
        };
        assert_eq!(object_type.get_name(), "Bookmark");

        let obtained_object_type = space.obtain_object_type(&spec).await.unwrap();
        assert_eq!(object_type.id(), obtained_object_type.id());
    })
    .await;
}
