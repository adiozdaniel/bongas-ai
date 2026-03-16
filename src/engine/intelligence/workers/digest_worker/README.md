# ✉️ Digest Worker: Automated Retention Engine

The Digest Worker is a background intelligence task responsible for re-engaging dormant users through hyper-personalized outbound discovery.

---

## 🚀 Lifecycle

1. **Scan**: Periodically scans for inactive profiles (e.g., users who haven't engaged in > 3 days).
2. **Execute**: Runs the `email_digest` scenario for each dormant user in a "headless" background context.
3. **Reason**: Triggers the `ReasoningWorker` logic to generate human-readable justifications for the picks.
4. **Package**: Wraps the top recommendations into an `EmailPayload`.
5. **Dispatch**: Hands off the payload to the `NotificationDispatcher` for delivery.

---

## 🎯 Strategic Impact

- **Closed-Loop Retention**: Turns the discovery engine from a reactive API into a proactive retention machine.
- **Zero Latency Impact**: Executes on the **Backstage Plane**, ensuring that large-scale digest generation never slows down active users.
- **Data Ownership**: Entirely automated within the sovereign VPC, using local logs to drive engagement without third-party data leakage.

---

[🏠 Hub](../../../../../docs/HUB.md) | [⬅️ Back to Workers Main](../README.md)
