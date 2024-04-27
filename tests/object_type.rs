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
            .with_network_sync(NetworkSync::NoSync)
            .with_root_path(temp_dir_path)
            .create_account("Test Client")
            .await
            .unwrap();

        let space = client.default_space().await.unwrap().unwrap();
        let spec = ObjectTypeSpec {
            name: "Bookmark".to_string(),
            recommended_relations: BTreeSet::from([
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
        assert_eq!(object_type.name(), "Bookmark");
        object_type
            .recommended_relations()
            .iter()
            .for_each(|relation| {
                assert!(spec
                    .recommended_relations
                    .contains(&relation.clone().into_spec()))
            });

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
            .with_network_sync(NetworkSync::NoSync)
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
            recommended_relations: BTreeSet::from([
                tag_relation.into_spec(),
                description_relation.into_spec(),
                source_relation.into_spec(),
            ]),
        };

        let object_type = match space.get_object_type(&spec).await.unwrap() {
            Some(object_type) => object_type,
            None => panic!("Bookmark object type doesn't exist on a new space"),
        };
        assert_eq!(object_type.name(), "Bookmark");
        object_type
            .recommended_relations()
            .iter()
            .for_each(|relation| {
                assert!(spec
                    .recommended_relations
                    .contains(&relation.clone().into_spec()))
            });

        let obtained_object_type = space.obtain_object_type(&spec).await.unwrap();
        assert_eq!(object_type.id(), obtained_object_type.id());
    })
    .await;
}

#[tokio::test]
async fn object_type_fails_to_obtain_on_unmatched_recommended_relations() {
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

        let result = client.default_space().await.unwrap().unwrap().obtain_object_type(&ObjectTypeSpec {
            name: "Bookmark".to_string(),
            recommended_relations: BTreeSet::from([
                RelationSpec {
                    name: "Tag".to_string(),
                    format: RelationFormat::MultiSelect,
                },
            ]),
        }).await.unwrap_err();

        // TODO: This should be an enum variant of an error type we control instead of a string
        if !result.message().contains("ObjectType `Bookmark` exists but has different recommended relations from requested recommended relations") {
          panic!("Unexpected error on obtaining object {} {}", result.code(), result.message());
        }
    })
    .await;
}

#[tokio::test]
async fn object_type_can_obtain_a_new_one_with_preexisting_relations() {
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

        let spec = ObjectTypeSpec {
            name: "NewType".to_string(),
            recommended_relations: BTreeSet::from([RelationSpec {
                name: "Tag".to_string(),
                format: RelationFormat::MultiSelect,
            }]),
        };

        if space.get_object_type(&spec).await.unwrap().is_some() {
            unreachable!("NewType is now a default anytype object type");
        }

        let object_type = space.obtain_object_type(&spec).await.unwrap();
        assert_eq!(object_type.name(), "NewType");
        object_type
            .recommended_relations()
            .iter()
            .for_each(|relation| {
                assert!(spec
                    .recommended_relations
                    .contains(&relation.clone().into_spec()))
            });
    })
    .await;
}

#[tokio::test]
async fn object_type_can_obtain_a_new_one_with_new_relations() {
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

        let relation_spec = RelationSpec {
            name: "NewRelation".to_string(),
            format: RelationFormat::Text,
        };
        let spec = ObjectTypeSpec {
            name: "NewType".to_string(),
            recommended_relations: BTreeSet::from([relation_spec.clone()]),
        };

        if space.get_object_type(&spec).await.unwrap().is_some() {
            unreachable!("NewType is now a default anytype object type");
        }

        let object_type = space.obtain_object_type(&spec).await.unwrap();
        assert_eq!(object_type.name(), "NewType");
        object_type
            .recommended_relations()
            .iter()
            .for_each(|relation| {
                assert!(spec
                    .recommended_relations
                    .contains(&relation.clone().into_spec()))
            });

        let relation = space.get_relation(&relation_spec).await.unwrap();
        match relation {
            None => panic!("NewRelation was not created"),
            Some(relation) => {
                assert_eq!(relation.name(), "NewRelation");
                assert_eq!(*relation.format(), RelationFormat::Text);
            }
        }
    })
    .await;
}
