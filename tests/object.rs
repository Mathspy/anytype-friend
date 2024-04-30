mod utils;

use std::collections::{BTreeSet, HashMap, HashSet};

use anytype_friend::{
    AnytypeClient, NetworkSync, ObjectDescription, ObjectSpec, ObjectTypeSpec, RelationFormat,
    RelationSpec, RelationValue,
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
            (RelationValue::Object(a), RelationValue::Object(b)) => {
                a.into_iter()
                    .map(|object| object.id().clone())
                    .collect::<HashSet<_>>()
                    == b.into_iter()
                        .map(|object| object.id().clone())
                        .collect::<HashSet<_>>()
            }
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
            object.get(&description_relation).await.unwrap(),
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
                        RelationValue::Email("cool@email.me".to_string()),
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
            object.get(&text_relation).await.unwrap(),
            RelationValue::Text("text!".to_string())
        );
        assert_relations_eq!(
            object.get(&number_relation).await.unwrap(),
            RelationValue::Number(5.0)
        );
        assert_relations_eq!(
            object.get(&date_relation).await.unwrap(),
            RelationValue::Date(now)
        );
        assert_relations_eq!(
            object.get(&checkbox_relation).await.unwrap(),
            RelationValue::Checkbox(true)
        );
        assert_relations_eq!(
            object.get(&url_relation).await.unwrap(),
            RelationValue::Url("https://gamediary.dev".to_string())
        );
        assert_relations_eq!(
            object.get(&email_relation).await.unwrap(),
            RelationValue::Email("cool@email.me".to_string())
        );
        assert_relations_eq!(
            object.get(&phone_relation).await.unwrap(),
            RelationValue::Phone("(555)555-5555".to_string())
        );
    })
    .await;
}

#[tokio::test]
async fn object_fails_to_create_with_incorrect_relation_format() {
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
        let number_relation = space
            .obtain_relation(&RelationSpec {
                name: "Number Relation Test".to_string(),
                format: RelationFormat::Number,
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

        let object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "TestType".to_string(),
                recommended_relations: BTreeSet::new(),
            })
            .await
            .unwrap();

        let error = space
            .create_object(ObjectDescription {
                ty: object_type.clone(),
                name: "Test Object".to_string(),
                relations: HashMap::from([(
                    number_relation.clone(),
                    RelationValue::Text("text!".to_string()),
                )]),
            })
            .await
            .unwrap_err();

        // TODO: This should be an enum error
        assert!(error
            .message()
            .contains("Expected format doesn't match received format"));

        let error = space
            .create_object(ObjectDescription {
                ty: object_type,
                name: "Test Object".to_string(),
                relations: HashMap::from([(
                    email_relation.clone(),
                    RelationValue::Phone("sneaky@email.com".to_string()),
                )]),
            })
            .await
            .unwrap_err();

        // TODO: This should be an enum error
        assert!(error
            .message()
            .contains("Expected format doesn't match received format"));
    })
    .await;
}

#[tokio::test]
async fn object_can_create_with_object_relations() {
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
        let object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "ObjectTypeRelationTest".to_string(),
                recommended_relations: BTreeSet::new(),
            })
            .await
            .unwrap();
        let relation = space
            .obtain_relation(&RelationSpec {
                name: "ObjectRelationTest".to_string(),
                format: RelationFormat::Object {
                    types: BTreeSet::from([object_type.id()]),
                },
            })
            .await
            .unwrap();
        let sample_object = space
            .create_object(ObjectDescription {
                ty: object_type.clone(),
                name: "SampleObject".to_string(),
                relations: HashMap::new(),
            })
            .await
            .unwrap();
        let sample_object_2 = space
            .create_object(ObjectDescription {
                ty: object_type,
                name: "SampleObject 2".to_string(),
                relations: HashMap::new(),
            })
            .await
            .unwrap();

        let object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "TestType".to_string(),
                recommended_relations: BTreeSet::from([relation.as_spec()]),
            })
            .await
            .unwrap();

        let object = space
            .create_object(ObjectDescription {
                ty: object_type,
                name: "Test Object".to_string(),
                relations: HashMap::from([(
                    relation.clone(),
                    RelationValue::Object(vec![sample_object.clone(), sample_object_2.clone()]),
                )]),
            })
            .await
            .unwrap();

        assert_eq!(object.name(), "Test Object");
        assert_relations_eq!(
            object.get(&relation).await.unwrap(),
            RelationValue::Object(vec![sample_object.clone(), sample_object_2.clone(),])
        );
    })
    .await;
}

#[tokio::test]
async fn object_fails_to_create_with_incorrect_object_type_relations() {
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
        let correct_object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "CorrectObjectType".to_string(),
                recommended_relations: BTreeSet::new(),
            })
            .await
            .unwrap();
        let wrong_object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "WrongObjectType".to_string(),
                recommended_relations: BTreeSet::new(),
            })
            .await
            .unwrap();
        let relation = space
            .obtain_relation(&RelationSpec {
                name: "ObjectRelationTest".to_string(),
                format: RelationFormat::Object {
                    types: BTreeSet::from([correct_object_type.id()]),
                },
            })
            .await
            .unwrap();
        let correct_object = space
            .create_object(ObjectDescription {
                ty: correct_object_type.clone(),
                name: "SampleObject".to_string(),
                relations: HashMap::new(),
            })
            .await
            .unwrap();
        let wrong_object = space
            .create_object(ObjectDescription {
                ty: wrong_object_type,
                name: "SampleObject 2".to_string(),
                relations: HashMap::new(),
            })
            .await
            .unwrap();

        let object_type = space
            .obtain_object_type(&ObjectTypeSpec {
                name: "TestType".to_string(),
                recommended_relations: BTreeSet::from([relation.as_spec()]),
            })
            .await
            .unwrap();

        let err = space
            .create_object(ObjectDescription {
                ty: object_type,
                name: "Test Object".to_string(),
                relations: HashMap::from([(
                    relation.clone(),
                    RelationValue::Object(vec![correct_object.clone(), wrong_object.clone()]),
                )]),
            })
            .await
            .unwrap_err();

        // TODO: This should be an enum error
        assert!(err
            .message()
            .contains("Expected format doesn't match received format"));
    })
    .await;
}

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
        let object_type = space
            .create_object_type(&ObjectTypeSpec {
                name: "TestObjectType".to_string(),
                recommended_relations: BTreeSet::new(),
            })
            .await
            .unwrap();

        let spec = ObjectSpec {
            name: "TestObject".to_string(),
            ty: object_type.clone(),
        };
        let created_object = space
            .create_object(ObjectDescription {
                ty: spec.ty.clone(),
                name: spec.name.clone(),
                relations: HashMap::new(),
            })
            .await
            .unwrap();
        let object = match space.get_object(&spec).await.unwrap() {
            Some(object) => object,
            None => panic!("Due date relation doesn't exist on a new space"),
        };
        assert_eq!(created_object.id(), object.id());
        assert_eq!(object.name(), "TestObject");
        assert_eq!(object.ty().await.unwrap().id(), object_type.id());

        let obtained_object = space.obtain_object(&spec).await.unwrap();
        assert_eq!(object.id(), obtained_object.id());
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
        let object_type = space
            .create_object_type(&ObjectTypeSpec {
                name: "TestObjectType".to_string(),
                recommended_relations: BTreeSet::new(),
            })
            .await
            .unwrap();

        let spec = ObjectSpec {
            name: "TestObject".to_string(),
            ty: object_type.clone(),
        };
        if space.get_object(&spec).await.unwrap().is_some() {
            unreachable!("TestObject is now a default anytype object");
        }

        let object = space.obtain_object(&spec).await.unwrap();
        assert_eq!(object.name(), "TestObject");
        assert_eq!(object.ty().await.unwrap().id(), object_type.id());
    })
    .await;
}
