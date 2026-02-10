-- BONGAS-AI Seed Scenarios
-- Core JSONB pipeline definitions for recommendation scenarios

SET search_path TO bongas, public;

INSERT INTO scenario_configs (slug, name, description, pipeline, cache_ttl_seconds) VALUES
(
    'continue_watching',
    'Continue Watching',
    'Shows videos user started but did not finish (10-90% completion)',
    '{
        "stages": [
            {
                "type": "fetch_clickhouse_watch_progress",
                "params": {"min_completion": 0.1, "max_completion": 0.9, "limit": 50}
            },
            {
                "type": "enrich_time_remaining",
                "params": {}
            },
            {
                "type": "sort_by_recency",
                "params": {"field": "last_watched_at", "descending": true}
            },
            {
                "type": "limit",
                "params": {"count": 20}
            }
        ]
    }'::jsonb,
    300
),
(
    'for_you_personalized',
    'For You (Personalized)',
    'ML-powered personalized recommendations using ONNX-optimized models',
    '{
        "stages": [
            {
                "type": "onnx_inference",
                "params": {
                    "model_name": "two_tower_v3",
                    "model_format": "onnx",
                    "model_path": "models/onnx/two_tower_v3.onnx",
                    "top_k": 200,
                    "fallback_to_python": true,
                    "batch_size": 32
                }
            },
            {
                "type": "filter_already_watched",
                "params": {}
            },
            {
                "type": "boost_by_recency",
                "params": {"boost_factor": 1.2, "decay_hours": 48}
            },
            {
                "type": "diversify_genres",
                "params": {"max_same_genre": 3}
            },
            {
                "type": "sort_by_score",
                "params": {"descending": true}
            },
            {
                "type": "limit",
                "params": {"count": 50}
            }
        ],
        "fallback_stages": [
            {
                "type": "fetch_popular_content",
                "params": {"time_window_hours": 168, "limit": 50}
            }
        ]
    }'::jsonb,
    600
),
(
    'trending_now',
    'Trending Now',
    'Hottest content based on recent engagement',
    '{
        "stages": [
            {
                "type": "fetch_clickhouse_trending",
                "params": {"time_window_hours": 24, "min_views": 100, "limit": 100}
            },
            {
                "type": "onnx_inference_bandit",
                "params": {
                    "model_name": "bandit_linucb",
                    "model_format": "onnx",
                    "algorithm": "linucb",
                    "explore_rate": 0.1
                }
            },
            {
                "type": "filter_already_watched",
                "params": {}
            },
            {
                "type": "limit",
                "params": {"count": 30}
            }
        ]
    }'::jsonb,
    180
),
(
    'because_you_watched',
    'Because You Watched...',
    'Similar content recommendations using ONNX-optimized similarity models',
    '{
        "stages": [
            {
                "type": "fetch_recent_watches",
                "params": {"limit": 5, "hours": 168}
            },
            {
                "type": "onnx_inference_similarity",
                "params": {
                    "model_name": "content_similarity",
                    "model_format": "onnx",
                    "method": "cosine",
                    "top_k_per_seed": 10
                }
            },
            {
                "type": "filter_already_watched",
                "params": {}
            },
            {
                "type": "deduplicate",
                "params": {}
            },
            {
                "type": "sort_by_score",
                "params": {"descending": true}
            },
            {
                "type": "limit",
                "params": {"count": 40}
            }
        ]
    }'::jsonb,
    600
);
