use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct AnyTypeMap{
    inner_map: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl Default for AnyTypeMap {
    fn default() -> Self {
        Self::new()
    }
}

impl AnyTypeMap{
    pub fn new() -> Self {
        Self { inner_map: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Thread-safe insertion. Blocks other readers/writers momentarily during insertion.
    pub fn insert<T: Send + Sync + 'static>(&self, value: T) {
        // .write() acquires exclusive access across all threads
        if let Ok(mut guard) = self.inner_map.write() {
            guard.insert(TypeId::of::<T>(), Box::new(value));
        }
    }

    /// Thread-safe concurrent retrieval. 
    /// Returns a cloned copy of the pluged instance so the lock can be freed instantly.
    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        // .read() allows unlimited parallel readers across threads
        let guard = self.inner_map.read().ok()?;
        guard.get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .cloned() // Clone the value out so we can release the lock immediately
    }
}