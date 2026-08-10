use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type Handler = Arc<dyn Fn(Event) -> BoxFuture + Send + Sync>;
type HandlerMap = Arc<Mutex<HashMap<u64, Handler>>>;

#[derive(Debug, Clone)]
pub enum Event {
    Heartbeat,
    VmCreated { vm_id: String },
    VmDeleted { vm_id: String },
    VmStarted { vm_id: String },
    VmStopped { vm_id: String },
    NodeJoined { node_ip: String },
    NodeLeft { node_ip: String },
    StoragePoolCreated { pool_name: String },
    NetworkCreated { network_name: String },
}

/// A RAII handle that automatically unsubscribes the associated handler when
/// dropped, preventing memory leaks from accumulated subscriptions.
pub struct SubscriptionHandle {
    handlers: HandlerMap,
    id: u64,
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        if let Ok(mut h) = self.handlers.lock() {
            h.remove(&self.id);
        }
    }
}

pub struct EventBus {
    handlers: HandlerMap,
    next_id: Arc<Mutex<u64>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Register a new event handler. Returns a [`SubscriptionHandle`] — when
    /// the handle is dropped (or explicitly called), the handler is
    /// automatically removed.
    pub fn subscribe<F, Fut>(&self, handler: F) -> SubscriptionHandle
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler: Handler = Arc::new(move |event| Box::pin(handler(event)));
        let id = {
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };
        self.handlers.lock().unwrap().insert(id, handler);
        SubscriptionHandle {
            handlers: Arc::clone(&self.handlers),
            id,
        }
    }

    /// Publish an event to all currently registered handlers. Each handler
    /// runs in its own tokio task; errors are logged rather than silently
    /// dropped.
    pub async fn publish(&self, event: Event) {
        let handlers = self.handlers.lock().unwrap().clone();
        let mut handles = Vec::with_capacity(handlers.len());
        for handler in handlers.values() {
            handles.push(tokio::spawn(handler(event.clone())));
        }
        for handle in handles {
            if let Err(e) = handle.await {
                tracing::error!("Event handler task panicked: {e}");
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
