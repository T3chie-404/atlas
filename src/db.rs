use rocksdb::{Options, DB};
use std::sync::Arc;
use tokio::task;

#[derive(Clone)]
pub struct Database {
    db: Arc<DB>,
}

impl Database {
    // Initialize the database
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let db = Arc::new(DB::open_default(path)?);
        Ok(Self { db })
    }

    // Asynchronous method to insert data
    pub async fn insert(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db_clone = Arc::clone(&self.db);
        let key_owned = key.to_vec();
        let value_owned = value.to_vec();
        task::spawn_blocking(move || {
            db_clone.put(&key_owned, &value_owned)?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })
        .await?
    }

    // Asynchronous method to retrieve data
    pub async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let db_clone = Arc::clone(&self.db);
        let key_owned = key.to_vec();
        let result = task::spawn_blocking(move || {
            let value = db_clone.get(&key_owned)?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(value)
        })
        .await?;
        result
    }
}
