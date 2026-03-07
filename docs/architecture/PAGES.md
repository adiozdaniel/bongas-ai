# 🎨 Smart Pages: The SDUI Canvas & Brain 2.0

[🏠 Hub](../HUB.md) | [🏗️ Architecture](./SYMPHONY.md) | [👤 Identity](./IDENTITY.md) | [⚡ Streaming](./ORCHESTRATION.md)

---

## 🏛️ The Philosophy: Living Layouts

A "Page" in Bongas-AI is a **Dynamic Blueprint** that adapts in real-time. Instead of hardcoding what a user sees, admins define **Rules** and **Pools** of content that the engine resolves and optimizes based on user engagement, device context, and server-side look-ahead.

## 📐 The Navigation Mesh

The engine assembles a personalized application structure every time a user connects to the genesis endpoint (`GET /api/v1/recommendation/page/home`).

### 1. The Genesis Entry Point
The first request resolves the user's context and finds the appropriate landing page.
- **Resolver**: Matches `device_type` and `maturity_rating` to a `PageLayout`.
- **Navigation Events**: The SSE stream immediately emits `navigation` and `sub_navigation` events to unblock client-side routing.

### 2. The Manifest Event (Skeleton UI)
Immediately following navigation, the engine emits a `manifest` event. This tells the UI:
- **`total_rows`**: How many rows are coming in this batch.
- **`batch_size`**: The current concurrency limit.
- **`prewarming_active`**: Whether "Ghost" pre-warming is active for the next batch.
- **Impact**: The client can render exactly the right number of "Skeleton Loaders" before the content actually arrives.

```mermaid
graph TD
    A[Genesis Request] --> B{Page Resolver}
    B --> C[Fetch Landing Page Composition]
    B --> D[Assemble Nav Mesh]
    C & D --> E[SSE Stream]
    E --> F[Event: navigation]
    E --> G[Event: manifest]
    E --> H[Event: row (Parallel)]
```

## 🧠 The Brain: Algorithmic Reordering

Beyond manual admin ordering, Bongas-AI implements **Personalized Layouts**. The engine "learns" from every interaction to promote high-engagement content.

### 1. The Real-Time Feedback Loop
The **Ingestion Manager** streams processed activities (clicks, views, likes) into ClickHouse. The **PagesManager** periodically analyzes these to maintain engagement scores per `visitor_id`.

### 2. Algorithmic Stable Sort
When a user requests a page, the engine performs a **stable sort** of the composition.
- **Affinity Score:** High-engagement scenarios (like "Continue Watching" or "Preferred Genres") automatically bubble to the top.
- **Persistence:** High-affinity rows are given priority during the **Parallel Fan-Out**, ensuring they arrive at the client first.

## 📺 Presentation Directives (SDUI)

| Row Type | UI Style | Best For |
| :--- | :--- | :--- |
| `hero` | `carousel` | High-impact promotional content. |
| `list` | `horizontal` | Standard browsing experience. |
| `grid` | `standard` | Category exploration. |
| `billboard` | `tall_cards` | Ads or creator-focused rows. |

---

## 🚀 Next Steps

- View the [**API Reference contract**](../api/REFERENCE.md).
- Return to the [**Documentation Hub**](../HUB.md).

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-smart-pages-the-sdui-canvas--brain-20)
