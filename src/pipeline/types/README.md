# 📦 Pipeline: Types

The core type system for the recommendation engine. Defines the fundamental traits and data structures that enable pluggable and composable pipelines.

---

## 🏗️ Core Interfaces

```mermaid
classDiagram
    class PipelineStage {
        <<interface>>
        +execute(context, params, input) Result
        +input_type() StageDataKind
        +output_type() StageDataKind
    }
    class ScoredItem {
        +i32 item_id
        +f32 score
        +Value metadata
    }
```

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Pipeline Main](../README.md)
