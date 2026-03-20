# 🏎️ Worker: Ghost Execution

> **Predictive session-aware sequencing.**

The Ghost Execution worker anticipates the user's next interaction by analyzing their current session history. It runs a lightweight predictive reflex (using the `flow_head` model) and pre-populates the **Ghost Cache** in Redis to ensure sub-millisecond delivery of the "exact next video."

## 🔄 Workflow

1. **Activity Intake:** Receives `UserActivity` events from the ingestion pipeline.
2. **History Tracking:** Updates the user's rolling session history in the L1/L2 cache.
3. **Flow Prediction:** Runs inference to predict the most likely next items.
4. **Ghost Warming:** Caches these predictions in the Ghost Cache for immediate retrieval.

---

[🏠 Hub](../../../../../docs/HUB.md) | [⬅️ Back to Workers](../README.md)
