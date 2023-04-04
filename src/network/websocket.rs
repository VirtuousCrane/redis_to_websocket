use std::{net::{TcpListener, TcpStream}, sync::{mpsc::{Receiver, self, Sender}, Arc, Mutex}, thread::{JoinHandle, self}, error::Error};

use log::{warn, info};
use redis::Msg;
use tungstenite::{WebSocket, Message};

pub struct WebSocketHandler {
    addr: String,
}

struct WebSocketHandlerWorker {
    rx: Receiver<Msg>,
    addr: String,
    websocket_list: Arc<Mutex<Vec<WebSocket<TcpStream>>>>
}

impl WebSocketHandler {
    pub fn new(host: &str, port: i32) -> WebSocketHandler {
        let addr = format!("{}:{}", host, port);
        WebSocketHandler { addr }
    }
    
    pub fn init(&self, rx: Receiver<Msg>) -> Result<JoinHandle<()>, Box<dyn Error>> {
        let addr_clone = self.addr.clone();
        let thread_handle = thread::spawn(move || {
            WebSocketHandlerWorker::new(rx, addr_clone)
                .start();
        });
        
        Ok(thread_handle)
    }
}

impl WebSocketHandlerWorker {
    pub fn new(rx: Receiver<Msg>, addr: String) -> WebSocketHandlerWorker {
        WebSocketHandlerWorker {
            rx,
            addr,
            websocket_list: Arc::new(Mutex::new(Vec::new()))
        }
    }
    
    pub fn start(&self) {
        let _listener_thread_handle = self.start_listener_thread();
        self.start_send_loop();
    }
    
    fn start_listener_thread(&self) -> JoinHandle<()> {
        let addr_clone = self.addr.clone();
        let websocket_list = self.websocket_list.clone();
        
        thread::spawn(move || {
            let listener = match TcpListener::bind(addr_clone) {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to bind TCP Listener: {}", e.to_string());
                    return;
                }
            };
            
            for stream in listener.incoming() {
                let websocket = match tungstenite::accept(stream.unwrap()) {
                    Ok(ws) => ws,
                    Err(e) => {
                        warn!("Failed to accept WebSocket: {}", e.to_string());
                        continue;
                    }
                };
                
                let mut ws_list = match websocket_list.lock() {
                    Ok(w) => w,
                    Err(e) => {
                        warn!("Failed to poison Mutex Lock: {}", e.to_string());
                        continue;
                    }
                };
                info!("New WebSocket Connection");
                ws_list.push(websocket);
            }
        })
    }
    
    fn start_send_loop(&self) {
        loop {
            let msg = match self.rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to Receive Message: {}", e.to_string());
                    continue;
                }
            };
            
            let payload: String = match msg.get_payload() {
                Ok(p) => p,
                Err(e) => {
                    warn!("Failed to Parse Message Payload: {}", e.to_string());
                    continue;
                }
            };
            
            let mut ws_list = match self.websocket_list.lock() {
                Ok(w) => w,
                Err(e) => {
                    warn!("Failed to Poison Mutex Lock: {}", e.to_string());
                    continue;
                }
            };
            
            let payload_as_message = Message::Text(payload);
            let mut sockets_to_remove: Vec<usize> = Vec::new();
            for (i, ws) in ws_list.iter_mut().enumerate() {
                if let Err(e) = ws.write_message(payload_as_message.clone()) {
                    warn!("Failed to write message to WebSocket: {}", e.to_string());
                    info!("Removing Dead WebSocket Connection...");
                    sockets_to_remove.push(i);
                    continue;
                }
            }
            
            for idx in sockets_to_remove {
                ws_list.remove(idx);
            }
            
        }
    }
}