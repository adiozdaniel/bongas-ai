# Scenarios & The Intelligent Brain

## Overview

BONGAS-AI utilizes an "Intelligent Brain" architecture that decouples product identity from recommendation strategy. This enables dynamic routing, contextual overrides, and AI-driven optimization without code changes.

The brain is split into four primary components:
1. **Scenarios**: The immutable product identity/endpoint (e.g., "Home Feed").
2. **Pipelines**: Reusable recommendation strategies (logic blocks).
3. **Scenario Rules**: The routing logic that connects a Scenario to a Pipeline based on context.
4. **Rule Suggestions**: AI-generated optimization proposals from the Analytics Sidecar.

## Database Schema

The core of the system resides in these finalized tables:

```sql
-- The Strategy (Logic)
CREATE TABLE pipelines (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    definition JSONB NOT NULL,
    diversity_score FLOAT DEFAULT 0.5,
    coverage_impact VARCHAR(32) DEFAULT 'medium'
);

-- The Product (Identity)
CREATE TABLE scenarios (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    target_kpi VARCHAR(32) DEFAULT 'retention',
    initial_display_limit INTEGER DEFAULT 5,
    cache_ttl_seconds INTEGER DEFAULT 300,
    use_l2_cache BOOLEAN DEFAULT true
);

-- The Routing (Brain)
CREATE TABLE scenario_rules (
    id SERIAL PRIMARY KEY,
    scenario_id INTEGER REFERENCES scenarios(id) ON DELETE CASCADE,
    pipeline_id INTEGER REFERENCES pipelines(id) ON DELETE RESTRICT,
    priority INTEGER DEFAULT 100,
    condition JSONB NOT NULL, -- e.g., {"context.device_type": "tv"}
    is_active BOOLEAN DEFAULT true
);
```

## Strategic Routing

When a request arrives for a scenario (e.g., `home_feed`), the engine evaluates `scenario_rules` in priority order. The first rule whose `condition` matches the request context determines the pipeline to execute.

### Supported Condition Keys
- `context.device_type`: Match against user device (e.g., `tv`, `mobile`).
- `context.profile_id`: Target specific user segments.
- `context.maturity_rating`: Enforce content governance (e.g., `G`, `PG-13`).
- `time.hour`: Exact hour match (0-23).
- `time.hour_range`: Range match (e.g., `[22, 4]` for bedtime discovery).

## Pipeline Structure

A pipeline definition is an ordered list of composable stages:

```json
{
  "stages": [
    {
      "type": "hot_items",
      "params": { "limit": 50 }
    },
    {
      "type": "diversity_reranker",
      "params": { "window": 20, "diversity_factor": 0.8 }
    }
  ]
}
```

## Hot-Reload

The system supports atomic hot-reloading. When rules or pipelines are updated via the API, the engine performs an atomic swap of its in-memory strategy map.

```bash
POST /api/v1/scenarios/reload-all
POST /api/v1/scenarios/:slug/reload
```

## Management API

The management API allows for full CRUD operations on scenarios and their default strategies.

```
POST   /api/v1/scenarios          - Create scenario & default strategy
GET    /api/v1/scenarios          - List all active scenarios
GET    /api/v1/scenarios/:slug    - Get scenario details
PUT    /api/v1/scenarios/:slug    - Update scenario or its default strategy
DELETE /api/v1/scenarios/:slug    - Delete scenario (cascades cleanup)
```
