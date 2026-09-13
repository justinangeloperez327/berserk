use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    sync::Arc,
};

#[derive(Clone, Default)]
pub(crate) struct StateMap(HashMap<TypeId, Arc<dyn Any + Send + Sync>>);
impl fmt::Debug for StateMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StateMap").field(&self.0.len()).finish()
    }
}
impl StateMap {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(crate) fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> bool {
        let key = TypeId::of::<T>();
        if self.0.contains_key(&key) {
            return false;
        }
        self.0.insert(key, Arc::new(value));
        true
    }
    pub(crate) fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.0.get(&TypeId::of::<T>())?.as_ref().downcast_ref()
    }
}
