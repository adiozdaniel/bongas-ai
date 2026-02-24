# 📦 Database: Repositories

Implements the Repository Pattern to decouple business logic from raw SQL queries. Each repository handles a specific domain entity.

---

## 🏗️ Repository Hierarchy

```mermaid
graph TD
    Repos[Repositories] --> Scen[Scenario Repository]
    Repos --> Feat[Feature Repository]
    Repos --> User[User Repository]
    Repos --> Inter[Interaction Repository]
    Repos --> Cache[Cache Repository]
```

---
[⬅️ Back to Database Main](../README.md)
