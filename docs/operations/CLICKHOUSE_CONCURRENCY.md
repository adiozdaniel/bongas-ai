# 📊 ClickHouse Concurrency Verification

This document provides the SQL queries and operational steps to verify that Symphony 2.0 is indeed executing scenarios in parallel (Phase 2: Velocity).

## 1. Schema Update (Step 3)

Execute this command in your ClickHouse client to support request-level tracking:

```sql
ALTER TABLE user_events ADD COLUMN request_id String AFTER profile_id;
```

## 2. Concurrency Proof Query (Step 4)

This query identifies requests where multiple scenarios were processed within the same 500ms window, proving the parallel "Fan-Out" is working.

```sql
SELECT
    request_id,
    groupArray(scenario_slug) AS active_scenarios,
    min(created_at) AS start_ts,
    max(created_at) AS end_ts,
    max(created_at) - min(created_at) AS total_ms_span
FROM user_events
WHERE created_at > (toUnixTimestamp(now()) - 3600) -- Last hour
GROUP BY request_id
HAVING length(active_scenarios) > 1
   AND total_ms_span < 500 -- All scenarios finished within a tight window
ORDER BY total_ms_span ASC;
```

## 3. Jaeger/OTLP Trace Search (Step 5)

To visualize the parallel "Waterfall" in Jaeger:

1. **Service:** `bongas-ai`
2. **Operation:** `execute_scenario`
3. **Tags:** `request_id=YOUR_ID`

**Verification:** In the Jaeger UI, you should see multiple `execute_scenario` bars starting at almost the exact same microsecond, rather than being stacked one after another.
