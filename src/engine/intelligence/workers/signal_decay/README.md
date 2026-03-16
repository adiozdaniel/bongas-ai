# 🪵 Signal Decay Engine: Cost-Effective Retention

The Signal Decay Engine is a background intelligence task responsible for the automated pruning of stale behavioral data from the ClickHouse analytical ledger.

---

## 🚀 Lifecycle

1. **Pulse**: Periodically (daily by default) triggers a data decay cycle.
2. **Calculate**: Determines the Unix timestamp threshold based on the `retention_days` configuration (default: 90 days).
3. **Prune**: Dispatches an asynchronous `ALTER TABLE ... DELETE` mutation to ClickHouse to remove all user interaction records older than the threshold.
4. **Monitor**: Logs the mutation status for reconciliation by the Backstage monitoring plane.

---

## 🎯 Strategic Impact

- **Linear Infrastructure Costs**: Prevents storage costs from scaling infinitely with the user base by automatically "forgetting" irrelevant history.
- **Improved Training Relevance**: Ensures that ML models are trained on fresh, relevant behavioral data rather than outdated signals.
- **Compliance Ready**: Provides a mechanical foundation for satisfying data retention policies (GDPR/Right to be Forgotten).

---

[🏠 Hub](../../../../../docs/HUB.md) | [⬅️ Back to Workers Main](../README.md)
