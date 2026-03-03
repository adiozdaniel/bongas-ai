# ⚡ THE STAGE: Execution Pillar

The Execution pillar is the high-performance discovery path of the Bongas-AI engine. It is optimized for zero-latency scenario resolution and parallel streaming.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🛰️ Core**](./core/README.md) | The high-performance execution loop and strategy resolver. |
| [**🧊 Cache**](./cache/README.md) | Predictive warming and L2 staging management. |
| [**⚙️ Runtime**](./runtime/README.md) | Request-scoped context and engine execution state. |

---

## 🎯 Design Principles

- **Non-Blocking**: Every operation must be optimized for the Tokio runtime.
- **Fast-Path**: Minimal allocation and zero-touch context propagation.
- **Resilient**: Guarded by global circuit breakers at the repository layer.
