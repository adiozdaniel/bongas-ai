# 🧪 API V1: Experiments

Interfaces for A/B testing and feature flagging. Enables the dynamic assignment of users to different recommendation variants for performance evaluation.

---

## 🏗️ Experiment Assignment

```mermaid
graph LR
    Req[User Request] --> Coord[Experiment Coordinator]
    Coord -->|Lookaside| Rule[Assignment Rule]
    Rule -->|Bucket| Variant[Test Variant]
    Variant --> Engine[Engine Execution]
```

---
[⬅️ Back to V1 Main](../README.md)
