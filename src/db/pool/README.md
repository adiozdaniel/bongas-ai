# 🏢 Database: Resilient Pool

Wraps `sqlx::Pool` with a circuit breaker to provide fast-fail capabilities when the database is overloaded or unavailable.

---

## 🛠️ Execution Flow

```mermaid
sequenceDiagram
    participant App
    participant Pool as Resilient Pool
    participant CB as Circuit Breaker
    participant DB as PostgreSQL
    
    App->>Pool: execute(query)
    Pool->>CB: call(query)
    CB->>DB: execute()
    alt DB Success
        DB-->>CB: OK
        CB-->>Pool: OK
        Pool-->>App: OK
    else DB Fail
        DB-->>CB: Err
        CB->>CB: Record failure
        CB-->>Pool: Err
        Pool-->>App: Err
    end
```

---
[⬅️ Back to Database Main](../README.md)
