# 📖 API Reference: The Unified Contract

[🏠 Hub](../HUB.md) | [🏗️ Architecture](../architecture/SYMPHONY.md) | [👤 Identity](../architecture/IDENTITY.md) | [⚡ Streaming](../architecture/ORCHESTRATION.md)

---

## 🔗 The Master Endpoint: `/page/{slug}`

In the new Bongas-AI architecture, this is the undisputed center of content delivery. All legacy endpoints (like `/trending` or `/home`) have been deprecated in favor of this unified streaming contract.

**Method:** `GET`  
**Endpoint:** `/api/v1/page/{page_slug}/{user_id}`

### 📥 Request Parameters

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `page_slug` | String | Yes | The slug of the page layout (e.g., `home`, `movies`). |
| `user_id` | Integer | Yes | The ID of the user (use `0` for anonymous). |
| `profile_id` | String | No | The specific profile within an account. |
| `device_type` | String | No | `mobile`, `tv`, `web`, `tablet`. |
| `maturity_rating`| String | No | KFCB Standards: `all`, `GE`, `PG`, `12`, `15`, `18`. |

### 🛠️ Extracted Context (Implicit)

The engine automatically extracts and propagates these fields from the request (No client action required):

| Parameter | Source | Description |
| :--- | :--- | :--- |
| `visitor_id` | Cookie | Persistent across sessions. |
| `device_hash`| IP + UA | Deterministic device fingerprint. |
| `ip_address` | Header | Client IP for regional targeting. |

---

## 📤 The Streaming Response (SSE)

The API returns an `EventSource` stream. Every event is a JSON payload.

### 1. Event: `navigation`

Sent instantly to build the app's navigation bar.

```json
{
  "active_pages": [
    {"slug": "home", "title": "Home"},
    {"slug": "movies", "title": "Movies"}
  ]
}
```

### 2. Event: `manifest`

Sent early to allow the client to render skeleton loaders.

```json
{
  "expected_rows": 5,
  "request_id": "uuid-v4",
  "page": "home"
}
```

### 3. Event: `row`

The actual content rows with presentation metadata.

```json
{
  "title": "Trending Now",
  "row_type": "hero_carousel",
  "row_style": "promotional",
  "scenario": "trending_now",
  "items": [...]
}
```

---

## 🛡️ Error & Safety Handling

- **Maturity Safety**: If a user's `maturity_rating` does not meet a scenario's global ceiling (e.g., a "GE" user requesting an "18" scenario), the engine emits an **SSE Comment** (`safety: restricted`) and skips the row.
- **Partial Failures**: If a single row fails during execution, the server emits an **SSE Comment** with the error.
- **Keep-Alive**: The server sends a `:` (comment) heartbeat every 15 seconds.

---

## 🚀 Next Steps

- Return to the [**Documentation Hub**](../HUB.md).
- Read about [**Identity & Context**](../architecture/IDENTITY.md).

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-api-reference-the-unified-contract)
