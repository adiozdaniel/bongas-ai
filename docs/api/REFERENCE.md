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
| `device_type` | String | No | KFCB standards detected automatically from UA. |
| `maturity_rating`| String | No | KFCB Standards: `all`, `GE`, `PG`, `12`, `15`, `18`. |

---

## 🏎️ Predictive Warming Endpoint

Triggers background execution for scenarios further down the layout to eliminate scroll-latency.

**Method:** `POST`  
**Endpoint:** `/api/v1/recommendations/prewarm`

### 📥 Request Body
```json
{
  "slugs": ["trending_now", "home_feed"],
  "user_id": 123
}
```

---

## 📤 The Streaming Response (SSE)

### 1. Event: `navigation`
Sent instantly to build the app's navigation bar.

### 2. Event: `manifest`
Sent early to allow the client to render skeleton loaders and plan pre-warming.
```json
{
  "expected_rows": 8,
  "request_id": "uuid-v4",
  "page": "home",
  "prewarm_scenarios": ["personalized_picks", "new_releases"]
}
```

### 3. Event: `row`
Individual content rows with fail-over protection. If a primary row fails, the engine automatically attempts its `fallback_slug`.

---

## 🛡️ Resilience & The Shield

- **Adaptive Rate Limiting**: Max 3 concurrent SSE connections per `visitor_id`. Rejections return `429 Too Many Requests`.
- **Fail-Over**: Automatic execution of fallback scenarios on primary failure.
- **Maturity Safety**: Early-block logic for KFCB compliance.

---

## 🚀 Next Steps

- Return to the [**Documentation Hub**](../HUB.md).
- Read about [**Identity & Context**](../architecture/IDENTITY.md).

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-api-reference-the-unified-contract)
