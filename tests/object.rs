mod utils;

use std::collections::{BTreeSet, HashMap};

use anytype_friend::{
    AnytypeClient, NetworkSync, ObjectDescription, ObjectTypeSpec, RelationFormat, RelationSpec,
    RelationValue,
};
use utils::run_with_service;

#[tokio::test]
async fn object_can_create_preexisting_one() {
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
                ty: object_type,
                name: "Test Object".to_string(),
                relations: HashMap::from([(
                    description_relation.clone(),
                    RelationValue::Text("We can create objects!".to_string()),
                )]),
            })
            .await
            .unwrap();

        assert_eq!(object.name(), "Test Object");
        assert_eq!(
            object.get(&description_relation),
            Some(RelationValue::Text("We can create objects!".to_string()),)
        );
    })
    .await;
}
