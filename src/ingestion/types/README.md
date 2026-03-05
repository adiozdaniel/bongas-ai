# 🧬 Ingestion: Types

Defines the core `UserActivity` domain model and the `ActivitySource` trait. This ensures a strictly typed contract between various input streams and the central processor.

---

## 🏗️ Domain Model

```mermaid
classDiagram
    class UserActivity {
        <<enumeration>>
        Impression
        Click
        Playback
        Conversion
    }
    class ActivitySource {
        <<interface>>
        +start(tx) Result
        +name() String
    }
```

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Ingestion Main](../README.md)
