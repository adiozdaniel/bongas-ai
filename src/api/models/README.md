# 🧬 API: Models & Symphony Context

Unified data structures for request and response payloads. These models ensure a consistent contract between the client (Canvas) and the server (Orchestrator).

[🏠 Hub](../../../docs/HUB.md) | [📖 API Reference](../../../docs/api/REFERENCE.md) | [👤 Identity](../../../docs/architecture/IDENTITY.md)

---

## 🏗️ Core Context Structures

### `ContextParams` (The Symphony State)

The most critical model in the Bongas-AI engine. It carries the environmental signals required for world-class personalization:

- **Identity**: `visitor_id` (Persistent), `device_hash` (Fingerprint).
- **Targeting**: `profile_id`, `maturity_rating`, `device_type`.
- **Regional**: `ip_address`.

This struct includes the **`merge_identity`** method, which seamlessly integrates context automatically extracted from the `identity_middleware`.

## 📦 Model Categories

| Category | Description |
| :--- | :--- |
| **Response** | Standard wrappers for success and error messages (`StandardResponse`). |
| **Recommendation** | Items (`RecommendationItem`), dynamic rows (`FeedRow`), and navigation models. |
| **Scenario** | Management payloads for creating/updating engine logic. |
| **System** | Contextual parameters and infrastructure metadata. |

---

[🏠 Hub](../../../docs/HUB.md) | [🔝 Top](#-api-models--symphony-context)
