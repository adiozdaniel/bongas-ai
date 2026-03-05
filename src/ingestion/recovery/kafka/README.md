# 🏗️ Ingestion Source: Kafka

The primary high-volume ingestion path. It consumes user interaction events from distributed Kafka topics, providing the scale needed for massive recommendation workloads.

---

## 🏗️ Consumer Architecture

```mermaid
graph TD
    Kafka[Kafka Clusters] -->|Topics| Group[Consumer Group]
    Group -->|Msg| Worker[Kafka Worker]
    Worker -->|Deserialize| activity[UserActivity]
    Worker -->|Push| Chan[Internal Channel]

    Worker -.-> CB[Circuit Breaker]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Recovery Main](../README.md)
