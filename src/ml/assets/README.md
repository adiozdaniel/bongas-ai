# 📦 THE PULSE: Assets Pillar

The Assets pillar manages the ML infrastructure, including model registries, loader statistics, and operational health.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🗄️ Registry**](./registry/README.md) | Versioned model storage and deployment governance. |
| [**📥 Loader**](./loader/README.md) | Hot-swapping logic and memory management. |
| [**🛠️ Utils**](./utils/README.md) | Shared mathematical and data utilities. |

---

## 🎯 Design Principles

- **Atomic Swapping**: Models are updated atomically without dropping inference requests.
- **Infrastructure Integrity**: Strict validation of model artifacts before registration.
- **Operational Visibility**: Real-time monitoring of model health and loader performance.
