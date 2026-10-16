//! One-line JSON records for failures and outcomes, on stderr: a timestamp, who wrote it, what
//! was being attempted, and the underlying error. Grep-able now, and easy to ship to a log
//! service later without changing the callers.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

/// Write one record. `fields` are added as they are (for example `status`, `path`, `round`).
pub fn event(level: &str, source: &str, what: &str, fields: Value) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut rec = Map::new();
    rec.insert("ts_ms".into(), json!(ts));
    rec.insert("level".into(), json!(level));
    rec.insert("source".into(), json!(source));
    rec.insert("what".into(), json!(what));
    if let Value::Object(extra) = fields {
        rec.extend(extra);
    }
    eprintln!("{}", Value::Object(rec));
}
