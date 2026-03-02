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
| `maturity_rating` | String | No | `G`, `PG`, `13+`, `18+`. |

### 🛠️ Extracted Context (Implicit)

The engine automatically extracts and propagates these fields from the request (No client action required):

| Parameter | Source | Description |
| :--- | :--- | :--- |
| `visitor_id` | Cookie | Persistent across sessions. |
| `device_hash` | IP + UA | Deterministic device fingerprint. |
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
  "layout_style": "default"
}
```

### 3. Event: `row`

The actual content rows.

```json
{
  "title": "Trending Now",
  "row_type": "horizontal_list",
  "scenario": "trending_now",
  "items": [...]
}
```

---

## 🛡️ Error Handling in Streams

- **Partial Failures**: If a single row fails, the server will emit an **SSE Comment** with the error details. The client should ignore this and keep the connection open for remaining rows.
- **Keep-Alive**: The server sends a `:` (comment) heartbeat every 15 seconds.

---

## 🚀 Next Steps

- Return to the [**Documentation Hub**](../HUB.md).
- Read about [**Identity & Context**](../architecture/IDENTITY.md).

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-api-reference-the-unified-contract)
