use rocksdb::{DB, Options};
use std::path::Path;

fn main() {
    // Replace this with your actual RocksDB database path
    let db_path = "/Users/ishamandaviya/Documents/atlas/temp/db";

    // Check if the path exists
    if !Path::new(db_path).exists() {
        eprintln!("Error: Database path '{}' does not exist.", db_path);
        std::process::exit(1);
    }

    // Open the database in read-only mode
    let mut options = Options::default();
    options.create_if_missing(false); // Don't create a new DB if it doesn't exist

    match DB::open_for_read_only(&options, db_path, false) {
        Ok(db) => {
            println!("Successfully opened RocksDB at '{}'", db_path);

            // Create an iterator to scan all key-value pairs
            let iter = db.iterator(rocksdb::IteratorMode::Start);

            // Iterate over all key-value pairs
            for item in iter {
                match item {
                    Ok((key, value)) => {
                        // Attempt to decode as UTF-8, fall back to hex if invalid
                        let key_str = String::from_utf8(key.to_vec())
                            .unwrap_or_else(|_| format!("{:x?}", key));
                        let value_str = String::from_utf8(value.to_vec())
                            .unwrap_or_else(|_| format!("{:x?}", value));

                        println!("Key: {}, Value: {}", key_str, value_str);
                    }
                    Err(e) => {
                        eprintln!("Error while iterating: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open RocksDB: {}", e);
            std::process::exit(1);
        }
    }
}