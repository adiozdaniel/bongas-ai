# 🛡️ Config Type: Circuit Breaker

Detailed parameters for the Hystrix-style resilience layer. Defines failure thresholds, window sizes, and state transition timeouts.

---

## 🏗️ Breaker Parameters

```mermaid
graph TD
    CB[CircuitBreakerConfig] --> Thresh[Failure Thresholds]
    CB --> Window[Sliding Window Settings]
    CB --> Wait[Wait Duration]
```

---
[⬅️ Back to Types Main](../README.md)
