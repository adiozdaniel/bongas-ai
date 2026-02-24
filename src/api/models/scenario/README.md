# 🎬 API Models: Scenario

Request payloads for managing recommendation scenarios. These models handle the input validation for creating and updating dynamic pipelines.

---

## 🏗️ Structure

```mermaid
classDiagram
    class CreateScenarioRequest {
        +String slug
        +PipelineDefinition pipeline
        +i32 cache_ttl
        +bool use_l2_cache
    }
    class UpdateScenarioRequest {
        +Option pipeline
        +Option cache_ttl
        +Option use_l2_cache
    }
```

---
[⬅️ Back to Models Main](../README.md)
