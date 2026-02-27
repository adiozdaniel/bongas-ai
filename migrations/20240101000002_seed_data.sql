-- BONGAS-AI Seed Data (Consolidated)
-- Includes Golden Strategies, Scenarios, and Initial Rules

SET search_path TO bongas, public;

-- 1. Seed Golden Strategies (Pipelines)
INSERT INTO pipelines (slug, name, definition, diversity_score, coverage_impact)
VALUES 
(
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
),
(
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
),
(
    'supreme_ranker_v1',
    'Supreme Ranker Strategy',
    '{
        "stages": [
            {"type": "hot_items", "params": {"limit": 100}},
            {"type": "vector_search", "params": {"limit": 50}},
            {"type": "onnx_ranker", "params": {"model": "ranker_v1", "batch_size": 16}}
        ]
    }'::jsonb,
    0.4,
    'medium'
)
ON CONFLICT (slug) DO NOTHING;

-- 2. Seed Core Scenarios
INSERT INTO scenarios (slug, name, description, category, target_kpi)
VALUES 
(
    'home_feed', 
    'Home Feed', 
    'Primary discovery feed for users', 
    'discovery', 
    'retention'
),
(
    'trending_now',
    'Trending Now',
    'Most popular content across the platform',
    'popularity',
    'conversion'
),
(
    'personalized_picks',
    'Personalized Picks',
    'AI-curated selection based on your taste',
    'personalized',
    'retention'
)
ON CONFLICT (slug) DO NOTHING;

-- 3. Seed Default Scenario Rules
INSERT INTO scenario_rules (scenario_id, pipeline_id, priority, condition, is_active, description)
SELECT 
    s.id, 
    p.id, 
    100, 
    '{}'::jsonb, 
    true, 
    'Default strategy for ' || s.name
FROM scenarios s
JOIN pipelines p ON (
    (s.slug = 'home_feed' AND p.slug = 'discovery_v1') OR
    (s.slug = 'trending_now' AND p.slug = 'supreme_ranker_v1') OR
    (s.slug = 'personalized_picks' AND p.slug = 'retention_v1')
)
ON CONFLICT DO NOTHING;

-- 4. Seed Contextual Overrides
INSERT INTO scenario_rules (scenario_id, pipeline_id, priority, condition, is_active, description)
SELECT 
    s.id, 
    (SELECT id FROM pipelines WHERE slug = 'supreme_ranker_v1'), 
    200, 
    '{"context.device_type": "tv"}'::jsonb, 
    true, 
    'High-performance ranker for Smart TVs'
FROM scenarios s
WHERE s.slug = 'home_feed'
ON CONFLICT DO NOTHING;

-- 5. Seed Page Layouts
INSERT INTO page_layouts (page_slug, scenario_slugs)
VALUES ('home', '["trending_now", "personalized_picks", "home_feed"]'::jsonb)
ON CONFLICT (page_slug) DO NOTHING;
