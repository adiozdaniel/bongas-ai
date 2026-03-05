# 🌐 Config Type: Server

Core web server settings for the Axum-based API. Handles host/port binding, TLS certificates, and connection keep-alive policies.

---

## 🏗️ Server Parameters

```mermaid
graph TD
    S[ServerConfig] --> B[Binding Address]
    S --> T[TLS / HTTPS]
    S --> C[Connection Limits]
    S --> K[Keep-Alive Timeouts]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🧬 Back to Types Main](../README.md) | [🔝 Top](#-config-type-server)
