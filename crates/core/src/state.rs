use std::{ops::Deref, sync::Arc};

/// Share one thread-safe value. Interior mutation remains explicit via locks.
#[derive(Debug)]
pub struct State<T: Send + Sync + 'static>(Arc<T>);

impl<T: Send + Sync + 'static> State<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}

// Manual implementation avoids requiring T: Clone.
impl<T: Send + Sync + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Send + Sync + 'static> Deref for State<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref()
    }
}
