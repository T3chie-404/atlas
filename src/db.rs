// db.rs
use rocksdb::{DB, ColumnFamilyDescriptor, Options, Error};
use std::path::Path;
use std::sync::Arc;

pub struct RocksDB {
    db: Arc<DB>,
    cf_name: String,
}

impl RocksDB {
    pub fn new(db_path: &str, cf_name: &str) -> Result<Self, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_max_open_files(512);

        let cf_descriptor = ColumnFamilyDescriptor::new(cf_name, Options::default());
        let db = Arc::new(DB::open_cf_descriptors(
            &opts,
            db_path,
            vec![cf_descriptor],
        )?);

        Ok(Self {
            db,
            cf_name: cf_name.to_string(),
        })
    }

    pub async fn process_command(&self, command: &str) -> String {
        let cf = self.db.cf_handle(&self.cf_name).unwrap();
        
        match command.split_once(':') {
            Some(("CREATE", fsid)) => {
                let db_clone = Arc::clone(&self.db);
                let fsid_clone = fsid.to_string();
                match tokio::task::spawn_blocking(move || {
                    db_clone.put_cf(cf, fsid_clone.as_bytes(), b"created")
                }).await {
                    Ok(Ok(_)) => format!("Directory '{}' created\n", fsid),
                    Ok(Err(e)) => format!("Error creating directory '{}': {}\n", fsid, e),
                    Err(e) => format!("Task failed: {}\n", e),
                }
            }
            Some(("DELETE", fsid)) => {
                let db_clone = Arc::clone(&self.db);
                let fsid_clone = fsid.to_string();
                match tokio::task::spawn_blocking(move || {
                    db_clone.delete_cf(cf, fsid_clone.as_bytes())
                }).await {
                    Ok(Ok(_)) => format!("Directory '{}' deleted\n", fsid),
                    Ok(Err(e)) => format!("Error deleting directory '{}': {}\n", fsid, e),
                    Err(e) => format!("Task failed: {}\n", e),
                }
            }
            _ => "Invalid command format. Use CREATE:FSID or DELETE:FSID\n".to_string(),
        }
    }

    pub fn get_db(&self) -> Arc<DB> {
        Arc::clone(&self.db)
    }
}