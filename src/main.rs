use std::{error::Error, sync::mpsc};

use env_logger::{Builder, Env};
use log::info;

use crate::network::{redis::RedisSubscriber, websocket::WebSocketHandler};
mod network;

fn main() {
    Builder::from_env(Env::default().default_filter_or("redis_to_websocket=trace"))
        .init();
    info!("Initialized logger");
    
    let redis_subscriber = RedisSubscriber::new("redis://127.0.0.1/");
    let websocket_handler = WebSocketHandler::new("localhost", 8888);
    
    let (tx, rx) = mpsc::channel();
    let websocket_thread_handle = match websocket_handler.init(rx) {
        Ok(w) => w,
        Err(_) => return,
    };
    
    let redis_thread_handle = match redis_subscriber.init(tx) {
        Ok(r) => r,
        Err(_) => return,
    };
    
    websocket_thread_handle.join();
    redis_thread_handle.join();
}
