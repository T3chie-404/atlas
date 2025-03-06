mod logger;
use logger::init_logger;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use log::{info, error};
use std::path::Path;

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
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(e) => {
                        error!("Error reading from socket: {:?}", e);
                        return;
                    }
                };

                let received_data = String::from_utf8_lossy(&buf[..n]).to_string();
                info!("Received: {}", received_data);

                let response = match received_data.trim().split_once(':') {
                    Some(("CREATE", fsid)) => {
                        let path = Path::new(fsid);
                        match fs::create_dir(&path).await {
                            Ok(_) => {
                                let msg = format!("Directory '{}' created", fsid);
                                let _ = tx.send(msg.clone());
                                format!("{}\n", msg)
                            }
                            Err(e) => format!("Error creating directory '{}': {}\n", fsid, e),
                        }
                    }
                    Some(("DELETE", fsid)) => {
                        let path = Path::new(fsid);
                        match fs::remove_dir_all(&path).await {
                            Ok(_) => {
                                let msg = format!("Directory '{}' deleted", fsid);
                                let _ = tx.send(msg.clone());
                                format!("{}\n", msg)
                            }
                            Err(e) => format!("Error deleting directory '{}': {}\n", fsid, e),
                        }
                    }
                    _ => "Invalid command format. Use CREATE:FSID or DELETE:FSID\n".to_string(),
                };

                if let Err(e) = socket.write_all(response.as_bytes()).await {
                    error!("Error writing to socket: {:?}", e);
                    return;
                }
            }
        });
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
        if let Err(e) = write.send(Message::Text(msg.into())).await {
            error!("Failed to send WebSocket message: {:?}", e);
            break;
        }
    }
}
