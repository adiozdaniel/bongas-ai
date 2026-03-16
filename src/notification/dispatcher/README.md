# 📡 Notification Dispatcher: Omnichannel Delivery Hub

The Notification Dispatcher is the central hub for orchestrating outbound engagement. It uses a pluggable adaptor pattern to support multiple delivery channels and environment-specific constraints.

---

## 🏛️ Adaptor Architecture

| Adaptor | Protocol | Use Case |
| :--- | :--- | :--- |
| **Resend** | HTTPS (API) | High-deliverability production email via [Resend](https://resend.com). |
| **Kafka** | Event Stream | Distributed, high-scale notification pipelines for mobile/web push. |
| **Polling** | Database (Local) | Local-only environments where delivery is pull-based via API endpoints. |

---

## 🚀 Key Features

### 1. Dynamic Templating (Handlebars)

The dispatcher integrates the **Handlebars** engine to perform server-side rendering of notification bodies. This allows the system to inject:

* **Semantic Why**: Personalization reasons (e.g., "Because you watched cyberpunk").
* **User Metadata**: Profile-specific attributes.
* **Discovery Payloads**: Top recommended item IDs.

### 2. Mandatory Persistence

Regardless of the chosen adaptor, every `NotificationIntent` is first persisted to:

1.**PostgreSQL**: `pending_emails` or `inbox_notifications` (System of Record).
2.**ClickHouse**: `notifications_ledger` (Analytical audit trail).

---

## ⚙️ Workflow

1.**Ingest**: Receive `NotificationIntent` from workers (e.g., `DigestWorker`).
2.**Render**: Apply Handlebars templates using provided metadata.
3.**Persist**: Save the intent to the Sovereign Ledger.
4.**Dispatch**: Handoff to the active adaptor (Resend, Kafka, or Polling).

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Notification Main](../README.md)
