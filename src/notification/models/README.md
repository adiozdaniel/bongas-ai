# 📦 Notification Models

Domain DTOs and database row representations for the Sovereign Outbound Engagement Hub.

---

## 🏛️ Data Structures

### 1. `NotificationIntent`

The primary enum representing a request to engage a user. Supports:

* **`Email(EmailPayload)`**: Transactional emails with dynamic templates.
* **`Inbox(InboxNotification)`**: In-app UI notifications.

### 2. `NotificationLedgerEntry`

Analytical row optimized for **ClickHouse**. Tracks the status, type, and timing of every engagement intent for ROI analysis.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Notification Main](../README.md)
