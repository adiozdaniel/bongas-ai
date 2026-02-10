# Scenarios

## Overview

Scenarios are the core abstraction in BONGAS-AI. Each scenario defines a recommendation pipeline as a JSONB document stored in PostgreSQL. This enables creating, modifying, and deploying new recommendation strategies without code changes or restarts.

## Scenario Schema

```sql
CREATE TABLE scenario_configs (
    slug VARCHAR(100) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    pipeline JSONB NOT NULL,
    priority INT DEFAULT 100,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

## Pipeline Structure

A pipeline is an ordered list of composable stages:

```json
{
  "slug": "personalized_continue_watching",
  "pipeline": {
    "stages": [
      {
        "type": "clickhouse_watch_progress",
        "params": { "min_completion": 0.15, "max_completion": 0.85 }
      },
      {
        "type": "ml_rerank",
        "params": { "model": "two_tower", "boost_factor": 1.2 }
      },
      {
        "type": "diversify_genres",
        "params": { "max_same_genre": 3 }
      },
      {
        "type": "filter_ignored",
        "params": {}
      },
      {
        "type": "limit_results",
        "params": { "count": 20 }
      }
    ]
  }
}
```

## Stage Categories

### Data Fetching
- `clickhouse_watch_progress` - Fetch 10-90% completed items
- `postgresql_content` - Fetch content by filters
- `redis_recent_interactions` - Fetch recent user actions
- `live_tv_tiered_fallback` - 3-tier live TV logic

### ML Inference
- `two_tower_inference` - Two-Tower model predictions
- `bert4rec_inference` - Sequential recommendations
- `collaborative_filtering` - ALS-based CF
- `content_based` - TF-IDF similarity

### Filtering
- `filter_by_genre` - Genre-based filtering
- `filter_by_age_rating` - Content rating filtering
- `filter_by_duration` - Min/max duration
- `filter_watched` - Exclude already watched
- `filter_ignored` - ProfileIgnoreRepository integration

### Boosting/Scoring
- `boost_trending` - Boost trending items
- `boost_new_content` - Boost new releases
- `boost_by_popularity` - Boost popular items
- `decay_by_age` - Time-based score decay

### Diversification
- `diversify_by_genre` - Ensure genre variety
- `diversify_by_creator` - Spread across creators
- `serendipity` - Inject surprising items

### Sorting/Limiting
- `sort_by_score` - Default score sorting
- `sort_by_recency` - Time-based sorting
- `limit_results` - Apply max limit
- `deduplicate` - Remove duplicates

## Hot-Reload

Scenarios can be reloaded without restart:

```
POST /api/v1/scenarios/reload-all
POST /api/v1/scenarios/:slug/reload
```

## CRUD Endpoints

```
POST   /api/v1/scenarios          - Create scenario
GET    /api/v1/scenarios          - List all scenarios
GET    /api/v1/scenarios/:slug    - Get scenario details
PUT    /api/v1/scenarios/:slug    - Update scenario
DELETE /api/v1/scenarios/:slug    - Delete scenario
```
