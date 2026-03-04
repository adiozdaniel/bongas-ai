# 🎼 THE CONDUCTOR: Coordination Pillar

The Coordination pillar is the assembly point of the Cortex. It orchestrates the interactions between Inference, Training, and Assets.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🧬 Service**](./service.rs) | The primary DiscoveryCortex implementation. |

---

## 🎯 Design Principles

- **Unified Intelligence**: Single point of coordination for all ML operations.
- **Resilient Wiring**: Injects circuit breakers and metrics across the cortex.
- **Context Preservation**: Ensures that request context is preserved across inference boundaries.
