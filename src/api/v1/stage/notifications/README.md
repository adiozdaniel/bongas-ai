# 📡 Notifications API

Stage endpoints for pulling personalized notifications and managing email dispatch.

---

## 🚀 Endpoints

### 1. `GET /notifications/inbox`

Retrieves the list of active, unread inbox notifications for a specific profile.

* **Query Params**: `profile_id` (required), `limit` (optional).
* **Purpose**: Populates the in-app notification center.

### 2. `GET /emails/pending`

Retrieves a batch of pending email payloads.

* **Query Params**: `limit` (optional).
* **Purpose**: Used by external high-speed mailers to poll for work.

### 3. `POST /emails/dispatched`

Marks a set of email IDs as successfully dispatched.

* **Payload**: `{"email_ids": [1, 2, 3]}`
* **Purpose**: Atomic hand-off to prevent duplicate email delivery.

---

[🏠 Hub](../../../../../docs/HUB.md) | [⬅️ Back to Stage Main](../README.md)
