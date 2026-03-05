# 🧱 Partial Results: Batch Error Handling

> **Resilient handling of non-atomic batch operations.**

The `PartialResult` type is used when an operation involves multiple items (like a batch fetch or a multi-stage pipeline) and we want to preserve successful results while identifying specific failures.

---

## 🏗️ Usage Pattern

```rust
let result: PartialResult<Item, Error> = service.process_batch(items).await;

if result.is_partial() {
    warn!("Batch completed with {} successes and {} failures", 
          result.succeeded.len(), 
          result.failed.len());
}
```

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Error Main Documentation](../README.md)
