# 🛠️ Middlewares: The Resilience Stack

> **Netflix-grade request orchestration, observability, and safety layers.**

The Middlewares module provides a pluggable stack of high-performance components that wrap our API endpoints. Each middleware is designed to enforce a specific aspect of system health, from load shedding to detailed telemetry.

---

## 🏗️ Architecture

The middleware stack is executed in a layered "onion" pattern. External requests pass through security and rate limiting first, before entering the resilience and business logic layers.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🛡️ Resilience**](./resilience/README.md) | Circuit breakers and timeout enforcement. |
| [**🧱 Bulkhead**](./bulkhead/README.md) | Resource isolation and concurrent request limits. |
| [**🚦 Rate Limit**](./rate_limit/README.md) | Adaptive throttling and quota management. |
| [**📊 Metrics**](./metrics/README.md) | Real-time telemetry and endpoint performance tracking. |
| [**🔐 Security**](./platform_security/README.md) | Request validation and platform-level safety checks. |
| [**🚨 Error Handling**](./unified_error/README.md) | Global error normalization and HTTP response mapping. |

---

[🏠 Hub](../../docs/HUB.md)
