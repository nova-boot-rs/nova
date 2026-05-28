use nova_boot_nosql::{NoSqlIndex, NovaNoSql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct UserDoc {
    id: String,
    email: String,
}

#[tokio::test]
async fn mongodb_upsert_get_delete_and_index_roundtrip() {
    let uri = match std::env::var("NOVA_TEST_MONGODB_URI") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Skipping MongoDB integration test: NOVA_TEST_MONGODB_URI is not set");
            return;
        }
    };

    let database =
        std::env::var("NOVA_TEST_MONGODB_DB").unwrap_or_else(|_| "nova_test".to_string());
    let collection = "users";

    let nosql = NovaNoSql::mongo_primary(&uri, &database)
        .await
        .expect("create mongo primary should succeed");

    let suffix = nanos_suffix();
    let user = UserDoc {
        id: format!("u-{suffix}"),
        email: format!("u{suffix}@nova.rs"),
    };

    nosql
        .upsert(collection, &user.id, &user)
        .await
        .expect("upsert should succeed");

    let loaded: Option<UserDoc> = nosql
        .get(collection, &user.id)
        .await
        .expect("get should succeed");
    assert_eq!(loaded, Some(user.clone()));

    let index_name = format!("users_email_idx_{suffix}");
    nosql
        .create_index(
            collection,
            NoSqlIndex::new(index_name.clone(), vec!["email".to_string()], true),
        )
        .await
        .expect("create index should succeed");

    let indexes = nosql
        .list_indexes(collection)
        .await
        .expect("list indexes should succeed");
    assert!(
        indexes.iter().any(|i| i.name == index_name),
        "expected custom email index to exist"
    );

    nosql
        .delete(collection, &user.id)
        .await
        .expect("delete should succeed");

    let after_delete: Option<UserDoc> = nosql
        .get(collection, &user.id)
        .await
        .expect("get after delete should succeed");
    assert!(after_delete.is_none(), "document should be deleted");
}

fn nanos_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
