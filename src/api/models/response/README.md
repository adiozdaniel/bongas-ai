# 🧬 API Models: Response

Defines the standard envelope for all API communications. Every endpoint returns a `StandardResponse` to ensure consistent error handling and metadata tracking for client applications.

---

## 🏗️ Envelope Structure

```mermaid
classDiagram
    class StandardResponse {
        +bool success
        +Option data
        +Option error
        +Option meta
    }
    class ErrorBody {
        +String message
        +String code
        +String classification
        +bool retriable
    }
    class ResponseMeta {
        +String request_id
        +DateTime timestamp
        +Option pagination
    }
    StandardResponse *-- ErrorBody
    StandardResponse *-- ResponseMeta
```

---
[⬅️ Back to Models Main](../README.md)
