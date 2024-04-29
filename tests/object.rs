mod utils;

use std::collections::{BTreeSet, HashMap};

use anytype_friend::{
    AnytypeClient, NetworkSync, ObjectDescription, ObjectTypeSpec, RelationFormat, RelationSpec,
    RelationValue,
};
use chrono::{DateTime, Utc};
use utils::run_with_service;

macro_rules! assert_relations_eq {
    ($a:expr, $b:expr) => {
        let equal = match ($a, $b) {
            (RelationValue::Text(a), RelationValue::Text(b))
            | (RelationValue::Url(a), RelationValue::Url(b))
            | (RelationValue::Email(a), RelationValue::Email(b))
            | (RelationValue::Phone(a), RelationValue::Phone(b)) => a == b,
            (RelationValue::Number(a), RelationValue::Number(b)) => a == b,
            (RelationValue::Date(a), RelationValue::Date(b)) => a == b,
            (RelationValue::Checkbox(a), RelationValue::Checkbox(b)) => a == b,
            _ => false,
        };

        if !equal {
            panic!(
                "assertion `left == right` failed
left: {:?}
right: {:?}",
                $a, $b
            )
        }
    };
}

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
        assert_relations_eq!(
            object.get(&description_relation).unwrap(),
            RelationValue::Text("We can create objects!".to_string())
        );
    })
    .await;
}

#[tokio::test]
async fn object_can_create_one_with_all_basic_relation_formats() {
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
        let text_relation = space
            .obtain_relation(&RelationSpec {
                name: "Text Relation Test".to_string(),
                format: RelationFormat::Text,
            })
            .await
            .unwrap();
        let number_relation = space
            .obtain_relation(&RelationSpec {
                name: "Number Relation Test".to_string(),
                format: RelationFormat::Number,
            })
            .await
            .unwrap();
        let date_relation = space
            .obtain_relation(&RelationSpec {
                name: "Date Relation Test".to_string(),
                format: RelationFormat::Date,
            })
            .await
            .unwrap();
        let checkbox_relation = space
            .obtain_relation(&RelationSpec {
                name: "Checkbox Relation Test".to_string(),
                format: RelationFormat::Checkbox,
            })
            .await
            .unwrap();
        let url_relation = space
            .obtain_relation(&RelationSpec {
                name: "Url Relation Test".to_string(),
                format: RelationFormat::Url,
            })
            .await
            .unwrap();
        let email_relation = space
            .obtain_relation(&RelationSpec {
                name: "Email Relation Test".to_string(),
                format: RelationFormat::Email,
            })
            .await
            .unwrap();
        let phone_relation = space
            .obtain_relation(&RelationSpec {
                name: "Phone Relation Test".to_string(),
                format: RelationFormat::Phone,
            })
            .await
            .unwrap();

        let object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "TestType".to_string(),
                recommended_relations: BTreeSet::from([
                    text_relation.as_spec(),
                    number_relation.as_spec(),
                    date_relation.as_spec(),
                    checkbox_relation.as_spec(),
                    url_relation.as_spec(),
                    email_relation.as_spec(),
                    phone_relation.as_spec(),
                ]),
            })
            .await
            .unwrap();

        let now = DateTime::from_timestamp(Utc::now().timestamp(), 0)
            .unwrap()
            .naive_utc();
        let object = space
            .create_object(ObjectDescription {
                ty: object_type,
                name: "Test Object".to_string(),
                relations: HashMap::from([
                    (
                        text_relation.clone(),
                        RelationValue::Text("text!".to_string()),
                    ),
                    (number_relation.clone(), RelationValue::Number(5.0)),
                    (date_relation.clone(), RelationValue::Date(now)),
                    (checkbox_relation.clone(), RelationValue::Checkbox(true)),
                    (
                        url_relation.clone(),
                        RelationValue::Url("https://gamediary.dev".to_string()),
                    ),
                    (
                        email_relation.clone(),
                        RelationValue::Url("cool@email.me".to_string()),
                    ),
                    (
                        phone_relation.clone(),
                        RelationValue::Phone("(555)555-5555".to_string()),
                    ),
                ]),
            })
            .await
            .unwrap();

        assert_eq!(object.name(), "Test Object");
        assert_relations_eq!(
            object.get(&text_relation).unwrap(),
            RelationValue::Text("text!".to_string())
        );
        assert_relations_eq!(
            object.get(&number_relation).unwrap(),
            RelationValue::Number(5.0)
        );
        assert_relations_eq!(
            object.get(&date_relation).unwrap(),
            RelationValue::Date(now)
        );
        assert_relations_eq!(
            object.get(&checkbox_relation).unwrap(),
            RelationValue::Checkbox(true)
        );
        assert_relations_eq!(
            object.get(&url_relation).unwrap(),
            RelationValue::Url("https://gamediary.dev".to_string())
        );
        assert_relations_eq!(
            object.get(&email_relation).unwrap(),
            RelationValue::Email("cool@email.me".to_string())
        );
        assert_relations_eq!(
            object.get(&phone_relation).unwrap(),
            RelationValue::Phone("(555)555-5555".to_string())
        );
    })
    .await;
}
