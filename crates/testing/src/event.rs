use berserk_events::{Event, Result};
use std::sync::{Arc, Mutex};

pub struct EventRecorder<E> {
    events: Arc<Mutex<Vec<E>>>,
}
impl<E> Clone for EventRecorder<E> {
    fn clone(&self) -> Self {
        Self {
            events: Arc::clone(&self.events),
        }
    }
}
impl<E: Clone + Event> EventRecorder<E> {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn listener(&self) -> impl Fn(&E) -> Result<()> + Send + Sync + 'static {
        let events = Arc::clone(&self.events);
        move |event| {
            events
                .lock()
                .expect("event recorder lock poisoned")
                .push(event.clone());
            Ok(())
        }
    }
    pub fn events(&self) -> Vec<E> {
        self.events
            .lock()
            .expect("event recorder lock poisoned")
            .clone()
    }
    pub fn count(&self) -> usize {
        self.events
            .lock()
            .expect("event recorder lock poisoned")
            .len()
    }
    pub fn last(&self) -> Option<E> {
        self.events
            .lock()
            .expect("event recorder lock poisoned")
            .last()
            .cloned()
    }
    pub fn clear(&self) {
        self.events
            .lock()
            .expect("event recorder lock poisoned")
            .clear();
    }
}
impl<E: Clone + Event> Default for EventRecorder<E> {
    fn default() -> Self {
        Self::new()
    }
}
