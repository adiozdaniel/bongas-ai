# API Reference

## Base URL

```
http://{host}:{port}/api/v1
```

## Recommendations

### Get Recommendations

```
GET /api/v1/recommendations/{scenario_slug}/{user_id}
```

Returns recommendations for a user based on the specified scenario pipeline.

**Parameters:**
- `scenario_slug` (path) - Scenario identifier (e.g., `personalized_home`, `continue_watching`)
- `user_id` (path) - User/profile ID

**Response:**
```json
{
  "items": [
    { "item_id": 123, "score": 0.95, "reason": "two_tower" },
    { "item_id": 456, "score": 0.87, "reason": "trending" }
  ],
  "scenario": "personalized_home",
  "cached": false,
  "latency_ms": 42
}
```

## Scenarios

### List Scenarios
```
GET /api/v1/scenarios
```

### Get Scenario
```
GET /api/v1/scenarios/{slug}
```

### Create Scenario
```
POST /api/v1/scenarios
Content-Type: application/json

{
  "slug": "trending_movies",
  "name": "Trending Movies",
  "description": "Movies trending in the last 24h",
  "pipeline": {
    "stages": [
      { "type": "clickhouse_trending", "params": { "window_hours": 24 } },
      { "type": "filter_by_genre", "params": { "genre": "movie" } },
      { "type": "limit_results", "params": { "count": 20 } }
    ]
  },
  "priority": 100,
  "enabled": true
}
```

### Update Scenario
```
PUT /api/v1/scenarios/{slug}
```

### Delete Scenario
```
DELETE /api/v1/scenarios/{slug}
```

### Reload Scenarios
```
POST /api/v1/scenarios/reload-all
POST /api/v1/scenarios/{slug}/reload
```

## Experiments

### List Experiments
```
GET /api/v1/experiments
```

### Create Experiment
```
POST /api/v1/experiments
Content-Type: application/json

{
  "name": "home_page_algo",
  "variants": ["two_tower_v1", "bert4rec_v1"],
  "strategy": "thompson_sampling",
  "traffic_percentage": 100
}
```

### Record Reward
```
POST /api/v1/experiments/{id}/reward
Content-Type: application/json

{ "variant": "two_tower_v1", "reward": 1.0 }
```

## Features

### Get User Features
```
GET /api/v1/features/user/{user_id}
```

### Get Item Features
```
GET /api/v1/features/item/{item_id}
```

## Analytics

### Trending
```
GET /api/v1/analytics/trending?window=24h&limit=50
```

### Watch Progress
```
GET /api/v1/analytics/watch-progress/{user_id}
```

## Common Response Format

All endpoints return:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

Error responses:

```json
{
  "success": false,
  "data": null,
  "error": { "code": "NOT_FOUND", "message": "Scenario not found" }
}
```
