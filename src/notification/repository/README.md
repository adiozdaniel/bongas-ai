# 🗄️ Notification Repository

The persistent heart of the engagement pillar. It ensures that every notification is securely stored before delivery and logged for real-time business intelligence.

---

## 🏛️ Persistence Strategy

The repository implements a **Dual-Write Pattern**:

### 1. System of Record (PostgreSQL)

Ensures transactional integrity for:

* `pending_emails`: Emails awaiting dispatch or marked as sent.
* `inbox_notifications`: The user's personalized in-app notification list.

### 2. Analytical Ledger (ClickHouse)

Captures high-throughput logs for:

* `notifications_ledger`: Every intent, status change, and delivery event. This enables the "Executive Command Center" to track engagement ROI.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Notification Main](../README.md)
