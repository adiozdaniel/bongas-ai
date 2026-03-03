# 🛰️ Core: Discovery Execution Path

The Core sub-module is the high-performance heart of the discovery engine. It handles scenario resolution and the main execution loop.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**⚡ Execution Manager**](./execution_manager/README.md) | High-performance recommendation execution loop and parallel processing. |
| [**🌲 Strategy Resolver**](./strategy_resolver/README.md) | Dynamic contextual routing and scenario rule matching. |

---

## 🎯 Design Principles

- **Zero-Latency Target**: Optimized for immediate response delivery.
- **Parallel Pipelining**: Executes multiple scenarios concurrently where possible.
- **Strict Isolation**: No side-effects during the discovery stream.
