# 📖 API Reference: The Bongas-AI Symphony

[🏠 Hub](../HUB.md) | [🏗️ Architecture](../architecture/SYMPHONY.md) | [👤 Identity](../architecture/IDENTITY.md) | [⚡ Streaming](../architecture/ORCHESTRATION.md)

---

## 🔗 Route Structure

The API is strictly divided between public content delivery and internal administration.

- **Public Base**: `/api/v1/recommendation`
- **Admin Base**: `/api/v1/recommendation/admin`

---

## 🌍 Public Endpoints (The Stage)

### 1. The Genesis Entry Point

The primary initiation call for the client application. It resolves the user's landing page and assembles the personalized navigation mesh.

**Method:** `GET`  
**Endpoint:** `/api/v1/recommendation`

#### 📥 Request Parameters

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `user_id` | Integer | Yes | User ID (use `0` for anonymous). |
| `profile_id` | String | No | Active profile ID. |
| `maturity_rating` | String | No | KFCB standards: `all`, `GE`, `PG`, `12`, `15`, `18`. |

---

### 2. The Page Orchestrator (Specific View)

Fetches a specific main or sub-page layout with support for batch-streaming.

**Method:** `GET`  
**Endpoint:** `/api/v1/recommendation/page/{slug}`

#### 📥 Page Request Parameters

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `offset` | Integer | No | Starting row index (Default: `0`). |
| `batch` | Integer | No | Number of rows to stream (Default: `5`). |

---

### 3. Frictionless Ingestion (Feedback)

Real-time behavior tracking to feed "The Brain." Automatically enriched by the server.

**Method:** `POST`  
**Endpoint:** `/api/v1/recommendation/ingest`

#### 📥 Ingest Request Body

```json
[
  { "event": "click", "item_id": 123, "scenario": "trending_now" },
  { "event": "playback", "item_id": 456, "watch_percentage": 0.85 }
]
```

---

### 4. Scenario Detail (See All)

Paginates items inside a specific row for grid views or infinite horizontal scrolls.

**Method:** `GET`  
**Endpoint:** `/api/v1/recommendation/scenario/{slug}`

---

## 📤 The SSE Event Lifecycle

Bongas-AI uses a multi-stage SSE stream to deliver an "Instant-On" experience.

| Event | Order | Description |
| :--- | :---: | :--- |
| `navigation` | 1 | Global main pages (Home, Movies, TV). |
| `sub_navigation` | 2 | Contextual sub-pages ranked by user engagement. |
| `manifest` | 3 | Metadata for the landing page (Expected rows, Trace ID). |
| `row` | 4+ | Individual content rows with SDUI metadata. |
| `continuation` | Last | The URL to fetch the **next batch** of rows. |

---

## 🛡️ Resilience & The Shield

- **Adaptive Rate Limiting**: Max 3 concurrent SSE connections per `visitor_id`.
- **Fail-Over**: Automatic execution of `fallback_slug` on primary scenario failure.
- **Maturity Safety**: Early-block logic ensures KFCB compliance.

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-api-reference-the-bongas-ai-symphony)
