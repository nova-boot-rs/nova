use crate::{error::NoSqlError, traits::DocumentStore, types::NoSqlIndex};
use async_trait::async_trait;
use futures_util::TryStreamExt;
use mongodb::{
    Client,
    bson::{Bson, Document, doc},
    options::IndexOptions,
};
use serde_json::Value as JsonValue;

/// Mongo adapter scaffold. This is intentionally lightweight until a full client is wired.
pub struct MongoDocumentStore {
    db: mongodb::Database,
}

impl MongoDocumentStore {
    pub async fn new(uri: impl AsRef<str>, database: impl AsRef<str>) -> Result<Self, NoSqlError> {
        let client = Client::with_uri_str(uri.as_ref())
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;

        Ok(Self {
            db: client.database(database.as_ref()),
        })
    }
}

#[async_trait]
impl DocumentStore for MongoDocumentStore {
    async fn get(&self, collection: &str, id: &str) -> Result<Option<JsonValue>, NoSqlError> {
        let col = self.db.collection::<Document>(collection);
        let found = col
            .find_one(doc! { "_id": id })
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;

        found
            .map(|d| serde_json::to_value(d).map_err(|e| NoSqlError::Serialization(e.to_string())))
            .transpose()
    }

    async fn upsert(&self, collection: &str, id: &str, doc: JsonValue) -> Result<(), NoSqlError> {
        let col = self.db.collection::<Document>(collection);

        let mut bson_doc: Document = mongodb::bson::to_document(&doc)
            .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
        bson_doc.insert("_id", Bson::String(id.to_string()));

        col.replace_one(doc! { "_id": id }, bson_doc)
            .upsert(true)
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), NoSqlError> {
        let col = self.db.collection::<Document>(collection);
        col.delete_one(doc! { "_id": id })
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn create_index(&self, collection: &str, index: NoSqlIndex) -> Result<(), NoSqlError> {
        let col = self.db.collection::<Document>(collection);

        let mut key_doc = Document::new();
        for field in index.fields {
            key_doc.insert(field, Bson::Int32(1));
        }

        let model = mongodb::IndexModel::builder()
            .keys(key_doc)
            .options(
                IndexOptions::builder()
                    .name(Some(index.name))
                    .unique(Some(index.unique))
                    .build(),
            )
            .build();

        col.create_index(model)
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn list_indexes(&self, collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError> {
        let col = self.db.collection::<Document>(collection);
        let mut cursor = col
            .list_indexes()
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;

        let mut out = Vec::new();
        while let Some(model) = cursor
            .try_next()
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?
        {
            let fields = model
                .keys
                .keys()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            let name = model
                .options
                .as_ref()
                .and_then(|o| o.name.clone())
                .unwrap_or_else(|| fields.join("_"));
            let unique = model
                .options
                .as_ref()
                .and_then(|o| o.unique)
                .unwrap_or(false);

            out.push(NoSqlIndex {
                name,
                fields,
                unique,
            });
        }

        Ok(out)
    }
}
