# 🧊 Cache: Discovery Hydration & Staging

The Cache sub-module ensures that discovery responses are consistently delivered at low latency through predictive hydration and L2 staging.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**☀️ Predictive Warmer**](./predictive_warmer/README.md) | Proactive cache hydration for likely discovery paths. |
| [**🗄️ Staging Manager**](./staging_manager/README.md) | L2 Cache orchestration and thundering herd protection. |

---

## 🎯 Design Principles

- **Predictive Performance**: Anticipates user navigation to hydrate discovery paths.
- **Thundering Herd Protection**: Single-flight resolution for cold cache keys.
- **Persistence Strategy**: Tiered L1 (In-Memory) and L2 (Redis) storage.
