# 🏗️ Config Type: Kafka

Communication settings for distributed stream processing. Defines broker addresses, consumer groups, and topic mappings for all user interaction streams.

---

## 🏗️ Stream Topography

```mermaid
graph TD
    Kafka[KafkaConfig] --> B[Brokers]
    Kafka --> G[Group IDs]
    Kafka --> T[Topic Manifest]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🧬 Back to Types Main](../README.md) | [🔝 Top](#️-config-type-kafka)
