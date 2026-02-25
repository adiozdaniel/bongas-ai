/// Result of a batch operation where some items may have failed.
#[derive(Debug, Clone)]
pub struct PartialResult<T, E> {
    pub succeeded: Vec<T>,
    pub failed: Vec<(T, E)>,
}

impl<T, E> PartialResult<T, E> {
    pub fn new(succeeded: Vec<T>, failed: Vec<(T, E)>) -> Self {
        Self { succeeded, failed }
    }

    pub fn all_succeeded(items: Vec<T>) -> Self {
        Self { succeeded: items, failed: Vec::new() }
    }

    pub fn all_failed(items: Vec<(T, E)>) -> Self {
        Self { succeeded: Vec::new(), failed: items }
    }

    pub fn is_complete_success(&self) -> bool {
        self.failed.is_empty()
    }

    pub fn is_complete_failure(&self) -> bool {
        self.succeeded.is_empty()
    }

    pub fn is_partial(&self) -> bool {
        !self.succeeded.is_empty() && !self.failed.is_empty()
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.succeeded.len() + self.failed.len();
        if total == 0 { 1.0 } else { self.succeeded.len() as f64 / total as f64 }
    }

    pub fn failed_items(self) -> Vec<T> {
        self.failed.into_iter().map(|(item, _)| item).collect()
    }
}
