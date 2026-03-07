# 📖 API Reference: The Bongas-AI Symphony 2.0

[🏠 Hub](../HUB.md) | [🏗️ Architecture](../architecture/SYMPHONY.md) | [👤 Identity](../architecture/IDENTITY.md) | [⚡ Streaming](../architecture/ORCHESTRATION.md)

---

## 🔗 Route Structure

The API is strictly divided between public content delivery and internal administration.

- **Public Base**: `/api/v1/recommendation`
- **Admin Base**: `/api/v1/recommendation/admin`

---

## 🛡️ The OTLP Shield (Headers)

Bongas-AI participates in distributed tracing. Clients or gateways should pass tracing headers to maintain observability.

| Header | Description |
| :--- | :--- |
| `traceparent` | Standard W3C Trace Context (e.g., `00-4bf92...-01`). |
| `X-Device-Type` | Optional. If missing, the server guesses from `User-Agent`. |
| `X-Profile-ID` | The active sub-profile (e.g., "Kids"). |

---

## 🌍 Public Endpoints (The Stage)

### 1. The Genesis Entry Point

The primary initiation call for the client application. It resolves the user's landing page and assembles the personalized navigation mesh.

**Method:** `GET`  
**Endpoint:** `/api/v1/recommendation/page/home`

#### 📥 Query Parameters

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `user_id` | Integer | No | Authenticated User ID. |
| `maturity_rating` | String | No | KFCB: `GE`, `PG`, `12`, `15`, `18`. |

---

### 2. The Page Orchestrator (Pagination)

Fetches a specific page layout with support for **Parallel Batch-Streaming**.

**Method:** `GET`  
**Endpoint:** `/api/v1/recommendation/page/{slug}`

#### 📥 Query

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `offset` | Integer | No | Starting row index (Default: `0`). |
| `batch` | Integer | No | Number of rows to stream (Default: 5). |

---

## 📤 The SSE Event Lifecycle

Symphony 2.0 uses a high-concurrency SSE stream. Events arrive in a specific logical order to unblock UI rendering.

| Event | Order | Payload Description |
| :--- | :---: | :--- |
| `navigation` | 1 | Global main pages (Home, Movies, TV). |
| `sub_navigation` | 2 | Contextual sub-pages ranked by affinity. |
| `manifest` | 3 | **Skeleton UI Hints**: `total_rows`, `batch_size`. |
| `row` | 4+ | **Content Row**: Scenario data + UI Presentation metadata. |
| `continuation` | Last | **Next URL**: The link to fetch the next batch. |

---

## 🧠 Brain & Ingestion

### Frictionless Ingestion

Real-time behavior tracking. The server automatically adds `visitor_id` and `request_id` to these events.

**Method:** `POST`  
**Endpoint:** `/api/v1/recommendation/ingest`

#### 📥 Example Body

```json
[
  { "event": "click", "item_id": 123, "scenario": "trending_now" },
  { "event": "playback", "item_id": 456, "watch_percentage": 0.85 }
]
```

---

## 🚀 Performance Features

- **Ghost Execution**: When calling `/page/home`, the server automatically pre-computes the next batch and stores it in Redis.
- **Parallel Fan-Out**: Up to 5 scenarios are executed simultaneously per SSE request.
- **Safe Pools**: Postgres and Redis are scaled to 100/50 connections with aggressive timeouts.

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-api-reference-the-bongas-ai-symphony-20)
