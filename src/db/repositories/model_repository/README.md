# 🧠 Database Repository: Model

> **Persistence and metadata management for ONNX machine learning models.**

The `model_repository` handles the storage and retrieval of model metadata, versioning, and deployment status. It ensures the engine can always find and load the correct weights for inference.

---

## 🏗️ Model Management Flow

```mermaid
graph LR
    Train[Training Pipeline] -->|Register| API[Admin API]
    API -->|Persist| Repo[Model Repository]
    Repo -->|Load| Engine[Bongas Engine]
    Engine -->|Inference| User[Final Recommendation]
```

---
[⬅️ Back to Repositories Main](../README.md)
