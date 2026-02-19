-- BONGAS-AI Phase 16: Data Migration (Revised)
-- Migrating from legacy scenario_configs to the Intelligent Brain schema.

SET search_path TO bongas, public;

-- 1. Extend scenarios table with configuration columns
ALTER TABLE scenarios
ADD COLUMN IF NOT EXISTS initial_display_limit INTEGER DEFAULT 5,
ADD COLUMN IF NOT EXISTS scope JSONB DEFAULT '{}'::jsonb,
ADD COLUMN IF NOT EXISTS cache_ttl_seconds INTEGER DEFAULT 300,
ADD COLUMN IF NOT EXISTS use_l2_cache BOOLEAN DEFAULT true,
ADD COLUMN IF NOT EXISTS category VARCHAR(100);

-- 2. Migrate Scenarios
INSERT INTO scenarios (slug, name, description, target_kpi, initial_display_limit, scope, cache_ttl_seconds, use_l2_cache, category, created_at)
SELECT 
    slug, 
    name, 
    description, 
    'retention' as target_kpi,
    initial_display_limit,
    scope,
    cache_ttl_seconds,
    use_l2_cache,
    category,
    created_at
FROM scenario_configs
ON CONFLICT (slug) DO UPDATE SET
    initial_display_limit = EXCLUDED.initial_display_limit,
    scope = EXCLUDED.scope,
    cache_ttl_seconds = EXCLUDED.cache_ttl_seconds,
    use_l2_cache = EXCLUDED.use_l2_cache,
    category = EXCLUDED.category;

-- 3. Migrate Pipelines
INSERT INTO pipelines (slug, name, definition, diversity_score, coverage_impact, created_at, updated_at)
SELECT 
    slug || '_strategy_v1', 
    name || ' Strategy', 
    pipeline, 
    0.5 as diversity_score, 
    'medium' as coverage_impact,
    created_at,
    updated_at
FROM scenario_configs
ON CONFLICT (slug) DO NOTHING;

-- 4. Link them in Scenario Rules
INSERT INTO scenario_rules (scenario_id, pipeline_id, priority, condition, is_active, description, created_at, updated_at)
SELECT 
    s.id as scenario_id,
    p.id as pipeline_id,
    100 as priority,
    '{}'::jsonb as condition,
    true as is_active,
    'Legacy Migration Rule' as description,
    NOW(),
    NOW()
-- 5. Seed Golden Strategies
INSERT INTO pipelines (slug, name, definition, diversity_score, coverage_impact)
VALUES (
    'discovery_v1', 
    'Global Discovery Strategy', 
    '{
        "stages": [
            {"type": "hot_items", "params": {"limit": 50}},
            {"type": "collaborative_filter", "params": {"limit": 50}},
            {"type": "diversity_reranker", "params": {"window": 20, "diversity_factor": 0.8}}
        ]
    }'::jsonb,
    0.8,
    'high'
)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO pipelines (slug, name, definition, diversity_score, coverage_impact)
VALUES (
    'retention_v1', 
    'Personalized Retention Strategy', 
    '{
        "stages": [
            {"type": "personalized_recommender", "params": {"model": "two_tower_v2", "limit": 100}},
            {"type": "staleness_filter", "params": {"max_impressions": 3}},
            {"type": "business_logic", "params": {"boost_recent": true}}
        ]
    }'::jsonb,
    0.3,
    'low'
)
ON CONFLICT (slug) DO NOTHING;
