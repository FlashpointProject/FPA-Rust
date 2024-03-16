use std::{collections::HashMap, sync::{mpsc, Arc, RwLock}};

use uuid::Uuid;

pub(crate) type LogEvent = String;
pub type SubscriptionId = Uuid;

pub(crate) struct EventManager {
    subscribers: RwLock<HashMap<SubscriptionId, mpsc::Sender<LogEvent>>>,
}

impl EventManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            subscribers: RwLock::new(HashMap::new()),
        })
    }

    pub fn subscribe(&self) -> (SubscriptionId, mpsc::Receiver<LogEvent>) {
        let (tx, rx) = mpsc::channel();
        let id = Uuid::new_v4();
        self.subscribers.write().unwrap().insert(id, tx);
        (id, rx)
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.subscribers.write().unwrap().remove(&id);
    }

    pub fn dispatch_event(&self, event: LogEvent) {
        let subscribers = self.subscribers.read().unwrap();
        for subscriber in subscribers.values() {
            println!("Sent - {}", event);
            let res = subscriber.send(event.clone()); // Ignoring send errors (e.g., if receiver is dropped)
            if res.is_err() {
                println!("Error sending - {:?}", res.unwrap_err());
            }
        }
    }
}
