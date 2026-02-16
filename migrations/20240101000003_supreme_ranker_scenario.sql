-- BONGAS-AI Phase 6: The Great Merging
-- Deployment of the Supreme Ranker (Grok-style multi-head inference)

SET search_path TO bongas, public;

INSERT INTO scenario_configs (slug, name, description, pipeline, cache_ttl_seconds) VALUES
(
    'supreme_ranker',
    'Supreme Ranker (Grok-Style)',
    'High-fidelity multi-head engagement prediction with weighted scoring and MMR diversification',
    '{
        "stages": [
            {
                "type": "fetch_popular_content",
                "params": {"time_window_hours": 168, "limit": 500}
            },
            {
                "type": "multi_action_ranker",
                "params": {
                    "model_name": "grok_engagement_v1",
                    "engagement_weights": {
                        "like": 5.0,
                        "reply": 10.0,
                        "retweet": 8.0,
                        "report": -50.0,
                        "dwell_time": 2.0
                    },
                    "head_mapping": {
                        "0": "like",
                        "1": "reply",
                        "2": "retweet",
                        "3": "report",
                        "4": "dwell_time"
                    },
                    "top_k": 200,
                    "batch_size": 128,
                    "user_feature_dim": 64,
                    "item_feature_dim": 32
                }
            },
            {
                "type": "diversify_mmr",
                "params": {
                    "lambda_param": 0.7,
                    "top_k": 50
                }
            },
            {
                "type": "sort_by_score",
                "params": {"descending": true}
            },
            {
                "type": "limit",
                "params": {"count": 40}
            }
        ],
        "fallback_stages": [
            {
                "type": "trending_now",
                "params": {}
            }
        ]
    }'::jsonb,
    60
);
