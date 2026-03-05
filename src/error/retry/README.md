# ⏳ Error Retry: Strategies & Hints

> **Intelligent backoff and recovery orchestration.**

The Retry module provides metadata to the resilience layer, allowing it to perform smart retries without hardcoding logic for every possible failure scenario.

---

## 🧩 Key Components

- **`BackoffStrategy`**: Defines the wait curve (Fixed, Exponential, Linear, None).
- **`RetryHint`**: A composite object containing the strategy, base delay, max retries, and jitter configuration.

---

## 🛠️ Backoff Patterns

| Strategy | Description | Use Case |
| :--- | :--- | :--- |
| **Exponential** | Progressively longer waits with jitter. | Network congestion, high-volume DBs. |
| **Fixed** | Constant interval between attempts. | Rate-limited APIs with known reset times. |
| **Immediate** | Retry without delay. | Local concurrency race conditions. |

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Error Main Documentation](../README.md)
