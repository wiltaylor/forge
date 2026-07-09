//! Small in-memory rate limiter for login attempts, keyed per account (works
//! regardless of reverse proxies, which hide client IPs anyway).

use std::collections::HashMap;
use std::sync::Mutex;

use crate::util::now;

const MAX_FAILURES: u32 = 10;
const WINDOW_SECS: i64 = 300;

#[derive(Default)]
pub struct LoginLimiter {
    // key → (failure count, window start)
    attempts: Mutex<HashMap<String, (u32, i64)>>,
}

impl LoginLimiter {
    /// True when this key has exhausted its failure budget.
    pub fn is_blocked(&self, key: &str) -> bool {
        let mut map = self.attempts.lock().unwrap();
        prune(&mut map);
        match map.get(key) {
            Some((count, start)) => *count >= MAX_FAILURES && now() - start < WINDOW_SECS,
            None => false,
        }
    }

    pub fn record_failure(&self, key: &str) {
        let mut map = self.attempts.lock().unwrap();
        let ts = now();
        let entry = map.entry(key.to_string()).or_insert((0, ts));
        if ts - entry.1 >= WINDOW_SECS {
            *entry = (0, ts);
        }
        entry.0 += 1;
    }

    pub fn record_success(&self, key: &str) {
        self.attempts.lock().unwrap().remove(key);
    }
}

fn prune(map: &mut HashMap<String, (u32, i64)>) {
    if map.len() > 10_000 {
        let cutoff = now() - WINDOW_SECS;
        map.retain(|_, (_, start)| *start >= cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_budget_and_resets_on_success() {
        let limiter = LoginLimiter::default();
        for _ in 0..MAX_FAILURES {
            assert!(!limiter.is_blocked("alice"));
            limiter.record_failure("alice");
        }
        assert!(limiter.is_blocked("alice"));
        assert!(!limiter.is_blocked("bob"));
        limiter.record_success("alice");
        assert!(!limiter.is_blocked("alice"));
    }
}
