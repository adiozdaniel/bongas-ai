# 📊 Circuit Breaker: Rolling Window

The `rolling_window` module provides a high-performance, bucketed metric storage system. It is responsible for tracking successes, failures, and latencies over a sliding time window.

---

## 🏗️ Metrics Design

```mermaid
graph LR
    subgraph Rolling Window
    B1[Bucket 1]
    B2[Bucket 2]
    B3[Bucket 3]
    B4[Bucket 4]
    end
    
    Time((Time)) -->|Advancing| B4
    Events((Events)) -->|Atomic Incr| B4
```

---

## 🔑 Key Features

- **Lock-Free Operation**: Uses atomic counters for zero-contention metrics updates.
- **Configurable Precision**: Adjustable window duration and bucket count.
- **Snapshot Support**: Provides immutable snapshots of metrics for decision-making without blocking writers.

---

[🏠 Hub](../../../docs/HUB.md) | [🛡️ Back to Circuit Breaker Main](../README.md) | [🔝 Top](#-circuit-breaker-rolling-window)
