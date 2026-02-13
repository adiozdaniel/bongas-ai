pub mod clickhouse;
pub mod kafka;
pub mod manager;
pub mod metrics;

pub use manager::ANALYTICS;

/*
Usage Example:
--------------
use crate::analytics::ANALYTICS;

ANALYTICS.kafka.messages_sent.with_label_values(&["user_events"]).inc();

let timer = ANALYTICS.db.query_latency.with_label_values(&["users"]).start_timer();
// ... run query ...
timer.observe_duration();
*/
