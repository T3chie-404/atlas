mod logger;
use std::path::Path;

use logger::init_logger;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message as TMessage;
use futures_util::{StreamExt, SinkExt};
use log::{info, error};
use protobuf::Message;


include!(concat!(env!("OUT_DIR"), "/test-protos/mod.rs"));

use types::{Request,Opcode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logger().expect("Failed to initialize logger");

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("TCP Server running on 127.0.0.1:8080");

    let ws_listener = TcpListener::bind("127.0.0.1:9000").await?;
    info!("WebSocket Server running on ws://127.0.0.1:9000");

    let (tx, _rx) = broadcast::channel::<String>(10);
    tokio::spawn(handle_websocket_clients(ws_listener, tx.clone()));

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let tx = tx.clone();

        info!("New TCP connection: {}", addr);
        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(e) => {
                        error!("Error reading from socket: {:?}", e);
                        return;
                    }
                };

                let received_data = &buf[..n];
                // let in_resp = Request::parse_from_bytes(&received_data).unwrap();
                match Request::parse_from_bytes(&received_data) {
                    Ok(request) => {
                        let response = handle_request(request, &tx).await;
                        if let Err(e) = socket.write_all(response.as_bytes()).await {
                            error!("Error writing to socket: {:?}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to decode Protobuf request: {:?}", e);
                        let _ = socket.write_all(b"Invalid Protobuf request\n").await;
                    }
                }
            }
        });
    }
}

// Function to handle Protobuf request
async fn handle_request(request: Request, tx: &broadcast::Sender<String>) -> String {
    
    match request.op.enum_value(){
        Ok(Opcode::BIGBANG) =>  {
            let pubkey_bytes = request.pubkey;

            // Attempt to convert 'pubkey' to a UTF-8 string
            match std::str::from_utf8(&pubkey_bytes) {
                Ok(fsid) => {
                    let path = Path::new(fsid);
                    match fs::create_dir(&path).await {
                        Ok(_) => {
                            let msg = format!("Directory '{}' created", fsid);
                            info!("{}", msg);
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
            ,
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
            ,
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
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use types::{Request, Opcode};

    #[tokio::test]
    async fn test_handle_request_bigbang() {
        let (tx, _rx) = broadcast::channel(10);
        let request = Request {
            op: Opcode::BIGBANG.into(),
            pubkey: b"GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L".to_vec(),
            ..Default::default()
        };
        let response = handle_request(request, &tx).await;
        assert_eq!(response, "Directory 'GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L' created");
    }
    #[tokio::test]
    async fn test_handle_request_armageddon() {
        let (tx, _rx) = broadcast::channel(10);
        let request = Request {
            op: Opcode::ARMAGEDDON.into(),
            pubkey: b"GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L".to_vec(),
            ..Default::default()
        };
        let response = handle_request(request, &tx).await;
        assert_eq!(response, "Directory 'GzuW7rkNfaLTvDb4vNEh2xr1e3GVgqrWgPMuBuckdu4L' deleted");
    }
}


