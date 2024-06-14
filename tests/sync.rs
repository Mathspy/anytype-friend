mod utils;

use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};

use anytype_friend::{
    AnytypeClient, NetworkSync, ObjectDescription, ObjectSpec, ObjectTypeSpec, RelationFormat,
    RelationSpec, RelationValue,
};
use utils::run_with_service;

#[tokio::test]
async fn can_sync() {
    let (mnemonic_tx, mnemonic_rx) = tokio::sync::oneshot::channel();
    let (object_created_tx, object_created_rx) = tokio::sync::oneshot::channel();

    let task_1 = tokio::spawn(run_with_service(|port| async move {
        let temp_dir_1 = tempdir::TempDir::new("anytype-friend").unwrap();
        let temp_dir_path_1 = temp_dir_1.path();

        let (mnemonic, client) = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::LocalOnly)
            .with_root_path(temp_dir_path_1)
            .create_account("Test Client")
            .await
            .unwrap();

        mnemonic_tx.send(mnemonic).unwrap();
        object_created_rx.await.unwrap();

        let space = client.default_space().await.unwrap().unwrap();
        let description_relation = space
            .obtain_relation(&RelationSpec {
                name: "Description".to_string(),
                format: RelationFormat::Text,
            })
            .await
            .unwrap();

        let object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "Bookmark".to_string(),
                recommended_relations: BTreeSet::from([
                    RelationSpec {
                        name: "Tag".to_string(),
                        format: RelationFormat::MultiSelect,
                    },
                    description_relation.as_spec(),
                    RelationSpec {
                        name: "Source".to_string(),
                        format: RelationFormat::Url,
                    },
                ]),
            })
            .await
            .unwrap();

        let object = space
            .obtain_object(&ObjectSpec {
                ty: object_type,
                name: "Test Object".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(object.name(), "Test Object");
        assert_relations_eq!(
            object.get(&description_relation).await.unwrap(),
            RelationValue::Text("We can create objects!".to_string())
        );

        dbg!(object.get(&description_relation).await.unwrap());

        object
            .set(
                &description_relation,
                RelationValue::Text("an update!!!".to_string()),
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_secs(60)).await;

        panic!("waited long enough");
    }));

    let task_2 = tokio::spawn(run_with_service(move |port| async move {
        let temp_dir_2 = tempdir::TempDir::new("anytype-friend").unwrap();
        let temp_dir_path_2 = temp_dir_2.path();

        let mnemonic = mnemonic_rx.await.unwrap();

        let client = AnytypeClient::connect(&format!("http://127.0.0.1:{port}"))
            .await
            .unwrap()
            .with_network_sync(NetworkSync::LocalOnly)
            .with_root_path(temp_dir_path_2)
            .authenticate(&mnemonic)
            .await
            .unwrap();

        let space = client.default_space().await.unwrap().unwrap();
        let description_relation = space
            .obtain_relation(&RelationSpec {
                name: "Description".to_string(),
                format: RelationFormat::Text,
            })
            .await
            .unwrap();

        let object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "Bookmark".to_string(),
                recommended_relations: BTreeSet::from([
                    RelationSpec {
                        name: "Tag".to_string(),
                        format: RelationFormat::MultiSelect,
                    },
                    description_relation.as_spec(),
                    RelationSpec {
                        name: "Source".to_string(),
                        format: RelationFormat::Url,
                    },
                ]),
            })
            .await
            .unwrap();

        let object = space
            .create_object(ObjectDescription {
                ty: object_type.clone(),
                name: "Test Object".to_string(),
                relations: HashMap::from([(
                    description_relation.clone(),
                    RelationValue::Text("We can create objects!".to_string()),
                )]),
            })
            .await
            .unwrap();

        assert_eq!(object.name(), "Test Object");

        object_created_tx.send(()).unwrap();

        loop {
            let object = space
                .obtain_object(&ObjectSpec {
                    ty: object_type.clone(),
                    name: "Test Object".to_string(),
                })
                .await
                .unwrap();

            let description = object.get(&description_relation).await.unwrap();
            let RelationValue::Text(description) = description else {
                tokio::task::yield_now().await;
                continue;
            };

            if description != "an update!!!" {
                tokio::task::yield_now().await;
                continue;
            }

            break;
        }
    }));

    tokio::select! {
        result = task_1 => {
            match result {
                Ok(_) => {},
                Err(err) => {
                    panic!("Task 1 failed with error: {err:?}")
                }
            }
        }
        result = task_2 => {
            match result {
                Ok(_) => {},
                Err(err) => {
                    panic!("Task 1 failed with error: {err:?}")
                }
            }

        }
    }
}
