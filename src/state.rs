use std::sync::Arc;

use crate::value::ValueService;

#[derive(Clone)]
pub struct AppState {
    pub value_service: Arc<ValueService>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            value_service: Arc::new(ValueService::new()),
        }
    }
}
