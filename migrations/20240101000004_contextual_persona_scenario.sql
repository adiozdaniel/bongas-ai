-- BONGAS-AI Phase 15: The Persona Era
-- Deployment of the Contextual Personalizer (Frozen-Once Strategy)

SET search_path TO bongas, public;

INSERT INTO scenario_configs (slug, name, description, pipeline, cache_ttl_seconds) VALUES
(
    'contextual_personalizer',
    'Contextual Personalizer',
    'Retrain-proof persona mapping based on device, time, and maturity rating',
    '{
        "stages": [
            {
                "type": "fetch_popular_content",
                "params": {"time_window_hours": 72, "limit": 200}
            },
            {
                "type": "maturity_filter",
                "params": {}
            },
            {
                "type": "ml_inference_two_tower",
                "params": {
                    "model_name": "frozen_persona_v1",
                    "top_k": 50,
                    "batch_size": 64
                }
            },
            {
                "type": "sort_by_score",
                "params": {"descending": true}
            },
            {
                "type": "limit",
                "params": {"count": 20}
            }
        ],
        "fallback_stages": [
            {
                "type": "trending_now",
                "params": {}
            }
        ]
    }'::jsonb,
    300
);
