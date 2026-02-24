# 🎯 API Models: Recommendation

Defines the core data structures for content delivery. These models are optimized for serialization and final presentation on client devices.

---

## 🏗️ Structure

```mermaid
classDiagram
    class HomeFeedResponse {
        +Vec rows
        +Option experiment_id
    }
    class FeedRow {
        +String title
        +String row_type
        +String scenario
        +Vec items
    }
    class RecommendationItem {
        +i32 item_id
        +String title
        +String thumbnail_url
        +f32 score
        +i32 rank
        +Value metadata
    }
    HomeFeedResponse *-- FeedRow
    FeedRow *-- RecommendationItem
```

---
[⬅️ Back to Models Main](../README.md)
