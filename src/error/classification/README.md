# 🎯 Error Classification

> **The strategy layer for resilience decision-making.**

This module defines how the system categorizes failures. Instead of resilience components (like the Circuit Breaker) knowing about specific database or network errors, they rely on the `ErrorClassification` provided by the domain.

---

## 🧩 Core Types

- **`ErrorClassification`**: An enum defining categories like `Transient`, `Permanent`, `Overload`, and `Timeout`.
- **`ErrorClassifier`**: The trait that domain errors implement to "teach" the system how to handle them.

---

## 🏗️ Classification Strategy

```mermaid
graph TD
    Error[Domain Error] -->|Implements| Trait[ErrorClassifier]
    Trait -->|classify()| Enum[ErrorClassification]
    Enum -->|Transient| CB[Trip Breaker / Retry]
    Enum -->|Permanent| Pass[Propagate to Caller]
    Enum -->|Overload| Shed[Load Shedding]
```

---
[⬅️ Back to Error Main](../README.md)
