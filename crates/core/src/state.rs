use std::{ops::Deref, sync::Arc};

/// Share one thread-safe value. Interior mutation remains explicit via locks.
#[derive(Debug)]
pub struct State<T: Send + Sync + 'static>(Arc<T>);

impl<T: Send + Sync + 'static> State<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }

    pub fn get_ref(&self) -> &T {
        self.0.as_ref()
    }

    pub fn into_arc(self) -> Arc<T> {
        self.0
    }

    /// Wrap an existing shared application value without cloning the value itself.
    pub fn from_arc(value: Arc<T>) -> Self {
        Self(value)
    }
}

// Manual implementation avoids requiring T: Clone.
impl<T: Send + Sync + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Send + Sync + 'static> From<T> for State<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Send + Sync + 'static> From<Arc<T>> for State<T> {
    fn from(value: Arc<T>) -> Self {
        Self::from_arc(value)
    }
}

impl<T: Send + Sync + 'static> AsRef<T> for State<T> {
    fn as_ref(&self) -> &T {
        self.get_ref()
    }
}

impl<T: Send + Sync + 'static> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.get_ref()
    }
}
