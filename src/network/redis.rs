use std::{thread::{JoinHandle, self}, sync::mpsc::Sender};

use log::warn;
use redis::{RedisResult, Client, Connection, Msg};

pub struct RedisSubscriber {
    redis_url: String,
}

struct RedisSubscriberWorker {
    connection: Connection,
    tx: Sender<Msg>
}

impl RedisSubscriber {
    pub fn new(redis_url: &str) -> RedisSubscriber {
        RedisSubscriber { redis_url: redis_url.into() }
    }
    
    pub fn init(&self, tx: Sender<Msg>) -> RedisResult<JoinHandle<()>> {
        let client = Client::open(self.redis_url.as_str())?;
        let connection = client.get_connection()?;
        
        Ok(self.spawn_thread(tx, connection))
    }
    
    fn spawn_thread(&self, tx: Sender<Msg>, connection: Connection) -> JoinHandle<()> {
        thread::spawn(move || {
            RedisSubscriberWorker::new(tx, connection)
                .start();
        })
    }
}

impl RedisSubscriberWorker {
    pub fn new(tx: Sender<Msg>, connection: Connection) -> RedisSubscriberWorker {
        RedisSubscriberWorker {
            connection,
            tx
        }
    }
    
    pub fn start(&mut self) {
        let mut pubsub = self.connection.as_pubsub();
        let sub_topics = vec!["tag:motioncapture.uwb", "tag:motioncapture.nodemcu.ARM-L", "tag:motioncapture.nodemcu.ARM-R", "tag:motioncapture.nodemcu.FOREARM-L", "tag:motioncapture.nodemcu.FOREARM-L", "tag:motioncapture.nodemcu.HEAD", "tag:motioncapture.nodemcu.LEG-L", "tag:motioncapture.nodemcu.LEG-R", "tag:motioncapture.nodemcu.THICC-L", "tag:motioncapture.nodemcu.THICC-R"]; 

        for topic in sub_topics {
            if let Err(e) = pubsub.subscribe(topic) {
                warn!("Failed to Subscribe to Redis: {}", e.to_string());
                return;
            }
        }
                
        loop {
            let msg = pubsub.get_message();
            let payload = match msg {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to receive message: {}", e.to_string());
                    continue;
                },
            };
            
            if let Err(e) = self.tx.send(payload) {
                warn!("Failed to send message: {}", e.to_string());
                continue;
            }
        }
    }
}
