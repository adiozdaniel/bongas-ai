# 🔍 Fetch: Search Results

The `FetchSearchResultsStage` is a Recovery Stage responsible for executing keyword-based queries against the Meilisearch index.

---

## 🛠 Usage

- **Stage Type**: `fetch_search_results`
- **Params**:
  - `index` (String, Optional): Meilisearch index name (defaults to "items").
  - `limit` (Usize, Optional): Number of results to fetch (default: 50).
  - `min_score` (F32, Optional): Minimum score threshold for results.

---

## 🚀 Execution Logic

1. **Extract**: Reads the search query (`q`) from the execution context (passed via query params).
2. **Retrieve**: Calls the Meilisearch client to fetch high-relevance candidate IDs.
3. **Hydrate**: Maps Meilisearch hits into `ScoredItem` DTOs with keyword matching metadata.
4. **Handoff**: Passes raw search results to subsequent Ranking stages for semantic re-ranking.

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Fetch Category](../README.md)
