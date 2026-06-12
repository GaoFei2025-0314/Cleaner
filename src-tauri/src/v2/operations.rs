use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct OperationToken {
    pub operation_id: String,
    pub cancelled: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct OperationRegistry {
    operations: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl OperationRegistry {
    pub fn register(&self) -> OperationToken {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.operations
            .lock()
            .expect("operation registry lock poisoned")
            .insert(operation_id.clone(), cancelled.clone());
        OperationToken {
            operation_id,
            cancelled,
        }
    }

    pub fn cancel(&self, operation_id: &str) -> bool {
        let Some(flag) = self
            .operations
            .lock()
            .expect("operation registry lock poisoned")
            .get(operation_id)
            .cloned()
        else {
            return false;
        };
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
        true
    }

    pub fn finish(&self, operation_id: &str) {
        self.operations
            .lock()
            .expect("operation registry lock poisoned")
            .remove(operation_id);
    }
}
