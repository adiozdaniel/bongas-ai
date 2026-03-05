# 📑 Error Context: Diagnostic Metadata

> **Rich tracing and observability for system failures.**

Error context allows the system to attach structured metadata to an error as it propagates through the stack, enabling precise root-cause analysis in distributed logs.

---

## 🧩 Structure

- **`request_id`**: Correlation ID for cross-service tracing.
- **`component`**: The architectural layer where the error originated (e.g., "pipeline", "cache").
- **`operation`**: The specific function or method call that failed.
- **`metadata`**: Arbitrary key-value pairs for domain-specific debugging data.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Error Main Documentation](../README.md)
