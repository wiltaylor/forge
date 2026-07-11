//! App-side validation book-keeping, mirroring forge-tui's `FormState`.
//! Widgets take errors through their `.error()` builder; this just holds the
//! field → message map.

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct FormState {
    errors: HashMap<String, String>,
}

impl FormState {
    pub fn error(&mut self, field: &str, msg: impl Into<String>) {
        self.errors.insert(field.to_owned(), msg.into());
    }

    pub fn error_of(&self, field: &str) -> Option<&str> {
        self.errors.get(field).map(String::as_str)
    }

    pub fn clear(&mut self, field: &str) {
        self.errors.remove(field);
    }

    pub fn clear_all(&mut self) {
        self.errors.clear();
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}
