# 🔔 Notification System: Sovereign Outbound Intelligence

The Notification system orchestrates multi-channel user engagement (Email and In-App) directly from the Bongas-AI engine. It is designed to drive retention through **"Intelligence-First"** engagement, ensuring no user telemetry ever leaves the sovereign VPC.

---

## 🏛️ Module Structure

The system is divided into three specialized sub-modules:

| Module | Responsibility | Location |
| :--- | :--- | :--- |
| **models** | Domain DTOs for emails, inbox alerts, and analytical ledger entries. | `models/` |
| **repository** | Mandatory persistence layer for PostgreSQL (S.O.R) and ClickHouse (Ledger). | `repository/` |
| **dispatcher** | The delivery hub using pluggable adaptors (Resend, Kafka, Polling). | `dispatcher/` |

---

## 🎯 Strategic Principles

- **Intelligence-First**: Notifications are not generic spam; they are driven by background workers (like `DigestWorker`) executing actual recommendation scenarios.
- **Sovereign Engagement**: By keeping the engagement logic and data in-house, we eliminate the "Growth Tax" of external SaaS providers and maintain absolute data privacy.
- **Environment Agnostic**: The system adapts to any infrastructure—from local polling for simple setups to Kafka-driven pipelines for global scale.
- **Dynamic Explainability**: Utilizes the **Handlebars** engine to inject "Semantic Why" reasons directly into notification bodies, building user trust.

---

## 🚀 Usage Flow

1. **Intent Creation**: A background worker creates a `NotificationIntent` (Email or Inbox).
2. **Dispatch**: The `NotificationDispatcher` receives the intent.
3. **Rendering**: Handlebars renders the final HTML/Message using provided metadata.
4. **Persistence**: The repository saves the intent to Postgres and logs it to ClickHouse.
5. **Delivery**: The active adaptor (e.g., `ResendNotifyAdaptor`) performs the physical delivery.

---

[🏠 Hub](../../docs/HUB.md) | [🔝 Top](#-notification-system-sovereign-outbound-intelligence)
