# 🚀 Boost: Hyper-Local Semantic Pulse

The Hyper-Local Semantic Pulse stage is a ranking component that dynamically boosts content matching physical regional events via semantic vector similarity.

---

## 🏎️ Execution Flow

1. **Location Resolution**: Extracts the user's location from the `ExecutionContext`.
2. **Pulse Retrieval**: Fetches active physical event themes and semantic vectors from Redis for that location.
3. **Maturity Guardrail**: Aborts the boost if the profile has a restricted maturity rating for the specific event theme (e.g., Natural Disasters are restricted for Kids/GE profiles).
4. **Semantic Matching**: Computes cosine similarity between the item embeddings and the regional event vector.
5. **Boost Application**: Applies a **1.3x boost** to items with a similarity score > 0.7.

---

## 🎯 Design Principles

- **Fail-Open**: Returns the candidate list unmodified if location is missing, Redis is down, or no active pulses are found.
- **Safety First**: Prevents potentially traumatic or sensitive regional content from being boosted to sensitive user segments.
- **Explainability**: Adds a reasoning entry to the `ScoredItem` for transparency: `Regional Boost: High community interest in [Theme] for this profile`.

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-boost-hyper-local-semantic-pulse)
