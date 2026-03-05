# 🧠 Config Type: ML

Settings for machine learning model management and inference. Controls model loading paths, ONNX runtime parameters, and training orchestrator behavior.

---

## 🏗️ ML Infrastructure

```mermaid
graph LR
    ML[MlConfig] --> Path[Model Registry Path]
    ML --> Opts[Runtime Threads]
    ML --> Train[Harvesting Interval]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🧬 Back to Types Main](../README.md) | [🔝 Top](#-config-type-ml)
