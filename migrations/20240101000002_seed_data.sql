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
INSERT INTO scenarios (slug, name, description, category, target_kpi, maturity_rating)
VALUES 
(
    'home_feed', 
    'Home Feed', 
    'Primary discovery feed for users', 
    'discovery', 
    'retention',
    'all'
),
(
    'trending_now',
    'Trending Now',
    'Most popular content across the platform',
    'popularity',
    'conversion',
    'all'
),
(
    'personalized_picks',
    'Personalized Picks',
    'AI-curated selection based on your taste',
    'personalized',
    'retention',
    '18'
)
ON CONFLICT (slug) DO NOTHING;

-- 3. Seed Default Scenario Rules
INSERT INTO scenario_rules (scenario_id, pipeline_id, priority, device_type, maturity_rating, condition, is_active, description)
SELECT 
    s.id, 
    p.id, 
    100, 
    'all',
    'all',
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
INSERT INTO scenario_rules (scenario_id, pipeline_id, priority, device_type, maturity_rating, condition, is_active, description)
SELECT 
    s.id, 
    (SELECT id FROM pipelines WHERE slug = 'supreme_ranker_v1'), 
    200, 
    'tv',
    'all',
    '{}'::jsonb, 
    true, 
    'High-performance ranker for Smart TVs'
FROM scenarios s
WHERE s.slug = 'home_feed'
ON CONFLICT DO NOTHING;

-- 5. Seed Page Layouts (The Symphony Navigation Mesh)
INSERT INTO page_layouts (page_slug, is_landing, nav_type, device_type, maturity_rating, priority, composition)
VALUES 
(
    'home', 
    true,   -- This is the entry point for default users
    'main', -- Always visible in primary nav
    'all', 
    'all', 
    100, 
    '[
        {"slug": "trending_now", "row_type": "hero_carousel", "row_style": "promotional", "fallback_slug": null},
        {"slug": "personalized_picks", "row_type": "horizontal_list", "row_style": "standard", "fallback_slug": "trending_now"},
        {"slug": "home_feed", "row_type": "horizontal_list", "row_style": "standard", "fallback_slug": null}
    ]'::jsonb
),
(
    'movies',
    false,
    'main',
    'all',
    'all',
    90,
    '[
        {"slug": "new_releases", "row_type": "hero_carousel", "row_style": "promotional", "fallback_slug": null},
        {"slug": "trending_now", "row_type": "horizontal_list", "row_style": "standard", "fallback_slug": null}
    ]'::jsonb
),
(
    'tv_shows',
    false,
    'main',
    'all',
    'all',
    80,
    '[
        {"slug": "home_feed", "row_type": "hero_carousel", "row_style": "promotional", "fallback_slug": null}
    ]'::jsonb
),
(
    'free_for_you',
    false,
    'sub', -- Contextual Hub
    'all',
    'all',
    50,
    '[
        {"slug": "personalized_picks", "row_type": "feature_grid", "row_style": "compact", "fallback_slug": null}
    ]'::jsonb
),
(
    'action_universe',
    false,
    'sub', -- Contextual Hub
    'all',
    'all',
    40,
    '[
        {"slug": "trending_now", "row_type": "horizontal_list", "row_style": "tall_cards", "fallback_slug": null}
    ]'::jsonb
)
ON CONFLICT (page_slug, device_type, maturity_rating) DO UPDATE 
SET is_landing = EXCLUDED.is_landing,
    nav_type = EXCLUDED.nav_type,
    composition = EXCLUDED.composition, 
    priority = EXCLUDED.priority;
