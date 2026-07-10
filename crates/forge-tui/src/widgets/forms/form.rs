use std::collections::BTreeMap;

/// Minimal form helper for apps not using the runtime's `FocusRing`: an
/// active-field index with wrap-around traversal plus per-field validation
/// messages. Fields are addressed by their render order index.
#[derive(Clone, Debug, Default)]
pub struct FormState {
    pub active: usize,
    pub len: usize,
    errors: BTreeMap<usize, String>,
}

impl FormState {
    pub fn new(len: usize) -> FormState {
        FormState { active: 0, len, errors: BTreeMap::new() }
    }

    pub fn is_active(&self, field: usize) -> bool {
        self.active == field
    }

    pub fn next(&mut self) {
        if self.len > 0 {
            self.active = (self.active + 1) % self.len;
        }
    }

    pub fn prev(&mut self) {
        if self.len > 0 {
            self.active = (self.active + self.len - 1) % self.len;
        }
    }

    pub fn set_error(&mut self, field: usize, message: impl Into<String>) {
        self.errors.insert(field, message.into());
    }

    pub fn clear_error(&mut self, field: usize) {
        self.errors.remove(&field);
    }

    pub fn error(&self, field: usize) -> Option<&str> {
        self.errors.get(&field).map(String::as_str)
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}
