# 🧠 Ranker: Hybrid Search Ranker

The `HybridSearchRankerStage` is a Ranking Stage responsible for balancing keyword-based scores from Meilisearch with semantic vector similarity scores for context-aware search results.

---

## 🛠 Usage

- **Stage Type**: `hybrid_search_ranker`
- **Params**:
  - `keyword_weight` (F32, Default: 0.4): Relative importance of exact keyword matches.
  - `semantic_weight` (F32, Default: 0.6): Relative importance of vector similarity or popularity heuristics.
  - `use_tribe` (Bool, Default: true): Whether to apply the user's Behavioral Tribe embedding for personalized re-ranking.

---

## 🚀 Re-Ranking Logic

**Hydrate**: Fetches full item features and embeddings from the `ItemFeatureService` for all candidate search results.

**Score**:

**Keyword Match**: Extracted from the `FetchSearchResultsStage`.

**Semantic Intent**:

Calculated using Cosine Similarity between the user's Behavioral Tribe and the item's latent vector.

**Aggregate**: Applies a weighted sum (Heuristic Aggregator) to produce the final `ScoredItem` score.

**Sort**: Re-orders the results to ensure high-intent, contextually relevant items appear first.

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Ranking Category](../README.md)
