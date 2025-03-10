mod logger;
mod db;
use std::path::Path;

use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use logger::init_logger;
use protobuf::Message;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message as TMessage;
use std::sync::Arc;
use tokio::task;
use rocksdb::DB;
use crate::db::Database;
include!(concat!(env!("OUT_DIR"), "/test-protos/mod.rs"));

use types::{Opcode, Request};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logger().expect("Failed to initialize logger");

    // Initialize the database
    let db = Database::new("/temp/db").unwrap();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("TCP Server running on 127.0.0.1:8080");

    let ws_listener = TcpListener::bind("127.0.0.1:9000").await?;
    info!("WebSocket Server running on ws://127.0.0.1:9000");

    let (tx, _rx) = broadcast::channel::<String>(10);
    tokio::spawn(handle_websocket_clients(ws_listener, tx.clone()));
    let db = db.clone(); // Clone the database handle for each connection

    if let Err(e) = db.insert("1".as_bytes(), b"Directory created").await {
        error!("Failed to insert record into RocksDB: {:?}", e);
    }
    Ok(())

    // loop {
    //     let (mut socket, addr) = listener.accept().await?;
    //     let tx = tx.clone();
    //     let db = db.clone(); // Clone the database handle for each connection

    //     info!("New TCP connection: {}", addr);
    //     tokio::spawn(async move {
    //         let mut buf = vec![0; 1024];

    //         loop {
    //             let n = match socket.read(&mut buf).await {
    //                 Ok(0) => return,
    //                 Ok(n) => n,
    //                 Err(e) => {
    //                     error!("Error reading from socket: {:?}", e);
    //                     return;
    //                 }
    //             };

    //             let received_data = &buf[..n];
    //             match Request::parse_from_bytes(&received_data) {
    //                 Ok(request) => {
    //                     let response = handle_request(request, &tx, &db).await;
    //                     if let Err(e) = socket.write_all(response.as_bytes()).await {
    //                         error!("Error writing to socket: {:?}", e);
    //                     }
    //                 }
    //                 Err(e) => {
    //                     error!("Failed to decode Protobuf request: {:?}", e);
    //                     let _ = socket.write_all(b"Invalid Protobuf request\n").await;
    //                 }
    //             }
    //         }
    //     });
    // }

}


// Function to handle Protobuf request
async fn handle_request(request: Request, tx: &broadcast::Sender<String>, db: &Database) -> String {
    match request.op.enum_value() {
        Ok(Opcode::BIGBANG) => {
            let pubkey_bytes = request.pubkey;

            // Attempt to convert 'pubkey' to a UTF-8 string
            match std::str::from_utf8(&pubkey_bytes) {
                Ok(fsid) => {
                    let path = Path::new(fsid);
                    match fs::create_dir(&path).await {
                        Ok(_) => {
                            let msg = format!("Directory '{}' created", fsid);
                            info!("{}", msg);

                            // let key = fsid.as_bytes().to_vec();
                            // let value = b"Directory created".to_vec();

                            // Perform the RocksDB insertion in a blocking-aware manner
                            // Insert record into RocksDB
                            if let Err(e) = db.insert(fsid.as_bytes(), b"Directory created").await {
                                error!("Failed to insert record into RocksDB: {:?}", e);
                            }

                            msg

                        }
                        Err(e) => {
                            let err_msg = format!("Error creating directory '{}': {}", fsid, e);
                            error!("{}", err_msg);
                            err_msg
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("Invalid UTF-8 sequence in 'pubkey': {}", e);
                    error!("{}", err_msg);
                    err_msg
                }
            }
        }
        Ok(Opcode::ARMAGEDDON) => {
            let pubkey_bytes = request.pubkey;

            // Attempt to convert 'pubkey' to a UTF-8 string

            match std::str::from_utf8(&pubkey_bytes) {
                Ok(fsid) => {
                    let path = Path::new(fsid);

                    match fs::remove_dir_all(&path).await {
                        Ok(_) => {
                            let msg = format!("Directory '{}' deleted", fsid);
                            let _ = tx.send(msg.clone());
                            msg
                        }
                        Err(e) => format!("Error deleting directory '{}': {}", fsid, e),
                    }
                }
                Err(e) => {
                    let err_msg = format!("Invalid UTF-8 sequence in 'pubkey': {}", e);
                    error!("{}", err_msg);
                    err_msg
                }
            }
        }
        Ok(Opcode::OPENRW) => "OPENRW".to_string(),
        Ok(Opcode::PEEK) => "PEEK".to_string(),
        Ok(Opcode::POKE) => "POKE".to_string(),
        Ok(Opcode::RM) => "RM".to_string(),
        Ok(Opcode::MKDIR) => "MKDIR".to_string(),
        Ok(Opcode::RMDIR) => "RMDIR".to_string(),
        Err(_) => "error".to_string(),
    }
}

// WebSocket Handler
async fn handle_websocket_clients(listener: TcpListener, tx: broadcast::Sender<String>) {
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("WebSocket accept failed: {:?}", e);
                continue;
            }
        };

        let rx = tx.subscribe();
        tokio::spawn(handle_ws_client(stream, rx));
    }
}

// Handle individual WebSocket clients
async fn handle_ws_client(stream: TcpStream, mut rx: broadcast::Receiver<String>) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("Failed to accept WebSocket connection: {:?}", e);
            return;
        }
    };

    let (mut write, _) = ws_stream.split();

    while let Ok(msg) = rx.recv().await {
        if let Err(e) = write.send(TMessage::Text(msg.into())).await {
            error!("Failed to send WebSocket message: {:?}", e);
            break;
        }
    }
}
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tokio::sync::broadcast;
//     use types::{Opcode, Request};


//     #[tokio::test]
//     async fn test_handle_request_bigbang() {
//         let (tx, _rx) = broadcast::channel(10);
//         let request = Request {
//             op: Opcode::BIGBANG.into(),
//             pubkey: b"GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L".to_vec(),
//             ..Default::default()
//         };
//         let db = Database::new("/temp/db").unwrap();

//         let response = handle_request(request, &tx, db).await;
//         assert_eq!(
//             response,
//             "Directory 'GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L' created"
//         );
//     }
//     #[tokio::test]
//     async fn test_handle_request_armageddon() {
//         let (tx, _rx) = broadcast::channel(10);
//         let request = Request {
//             op: Opcode::ARMAGEDDON.into(),
//             pubkey: b"GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L".to_vec(),
//             ..Default::default()
//         };
//         let db = Arc::new(MockDatabase::new());
//         let response = handle_request(request, &tx, &db).await;
//         assert_eq!(
//             response,
//             "Directory 'GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L' deleted"
//         );
//     }
// }
