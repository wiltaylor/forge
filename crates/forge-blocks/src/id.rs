//! Block id generation — ids only need to be unique strings; the web side
//! uses `crypto.randomUUID()`, this side a timestamp + process counter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh block id, e.g. `blk_18f2c4a1b3e_2a`.
pub fn new_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("blk_{millis:x}_{n:x}")
}
