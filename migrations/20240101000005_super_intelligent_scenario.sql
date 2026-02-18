-- BONGAS-AI Phase 15: The Super Intelligent Scenario
-- Deployment of the "Super Personalizer" using Branching, Ensemble, and Interleaving.

SET search_path TO bongas, public;

INSERT INTO scenario_configs (slug, name, description, pipeline, cache_ttl_seconds) VALUES
(
    'super_personalizer',
    'Super Intelligent Feed',
    'Advanced multi-strategy feed with persona branching and slot interleaving',
    '{
        "stages": [
            {
                "type": "branch",
                "params": {
                    "condition": { "key": "context.maturity_rating", "operator": "==", "value": "GE" },
                    "if_true": [
                        { "type": "fetch_by_category", "params": {"category": "Kids", "limit": 100} },
                        { "type": "boost_by_popularity", "params": {"factor": 2.0} }
                    ],
                    "if_false": [
                        {
                            "type": "ensemble",
                            "params": {
                                "sources": [
                                    {
                                        "name": "personalized",
                                        "weight": 0.8,
                                        "stages": [
                                            { "type": "fetch_user_preferences", "params": {"limit": 100} },
                                            { "type": "ml_inference_two_tower", "params": {"model_name": "frozen_persona_v1", "top_k": 50} }
                                        ]
                                    },
                                    {
                                        "name": "trending",
                                        "weight": 0.2,
                                        "stages": [
                                            { "type": "fetch_clickhouse_trending", "params": {"time_window_hours": 24, "limit": 50} }
                                        ]
                                    }
                                ]
                            }
                        }
                    ]
                }
            },
            {
                "type": "affinity_freshness",
                "params": {
                    "max_boost": 5.0,
                    "freshness_window_hours": 72,
                    "affinity_key": "genre"
                }
            },
            {
                "type": "interleave",
                "params": {
                    "pattern": ["personalized", "fresh", "personalized", "discovery"],
                    "sources": {
                        "personalized": [ { "type": "sort_by_score", "params": {} }, { "type": "limit", "params": {"count": 20} } ],
                        "fresh": [ { "type": "fetch_new_releases", "params": {"limit": 10} }, { "type": "affinity_freshness", "params": {"max_boost": 3.0} } ],
                        "discovery": [ { "type": "fetch_popular_content", "params": {"limit": 10} }, { "type": "diversify_mmr", "params": {"lambda_param": 0.5} } ]
                    }
                }
            },
            {
                "type": "sort_by_score",
                "params": {"descending": true}
            },
            { "type": "limit", "params": {"count": 20} }
        ]
    }'::jsonb,
    60
);
