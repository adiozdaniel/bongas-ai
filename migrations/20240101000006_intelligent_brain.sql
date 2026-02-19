-- BONGAS-AI Phase 16: The Intelligent Brain
-- Decoupling Identity, Strategy, and Governance for Autonomous Optimization.

SET search_path TO bongas, public;

-- 1. Pipelines: The Strategy (Reusable Logic blocks)
CREATE TABLE pipelines (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(64) UNIQUE NOT NULL, -- e.g., 'high_diversity_v1'
    name VARCHAR(128) NOT NULL,
    definition JSONB NOT NULL,        -- The actual pipeline stages
    diversity_score FLOAT DEFAULT 0.5,
    coverage_impact VARCHAR(32) DEFAULT 'medium', -- 'low', 'medium', 'high'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2. Scenarios: The Product (Immutable identity/endpoints)
CREATE TABLE scenarios (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(64) UNIQUE NOT NULL, -- e.g., 'home_feed'
    name VARCHAR(128) NOT NULL,
    description TEXT,
    target_kpi VARCHAR(32) DEFAULT 'retention', -- 'retention', 'diversity', 'conversion'
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. Scenario Rules: The Active Brain (Routing logic)
CREATE TABLE scenario_rules (
    id SERIAL PRIMARY KEY,
    scenario_id INTEGER REFERENCES scenarios(id) ON DELETE CASCADE,
    pipeline_id INTEGER REFERENCES pipelines(id) ON DELETE RESTRICT,
    priority INTEGER DEFAULT 100,
    condition JSONB NOT NULL, -- e.g., {"context.device_type": "tv", "time.hour_range": [20, 23]}
    is_active BOOLEAN DEFAULT true,
    description TEXT, -- Why this rule exists (e.g., "Bedtime discovery for TV")
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 4. Rule Suggestions: The Suggestion Box (AI/Sidecar output)
CREATE TABLE rule_suggestions (
    id SERIAL PRIMARY KEY,
    scenario_id INTEGER REFERENCES scenarios(id) ON DELETE CASCADE,
    suggested_pipeline_id INTEGER REFERENCES pipelines(id) ON DELETE CASCADE,
    suggested_condition JSONB NOT NULL,
    reasoning TEXT,
    confidence_score FLOAT CHECK (confidence_score >= 0 AND confidence_score <= 1),
    status VARCHAR(32) DEFAULT 'pending', -- 'pending', 'approved', 'rejected', 'auto_applied'
    auto_apply_threshold FLOAT DEFAULT 0.95,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    applied_at TIMESTAMPTZ
);

-- Indexes for performance
CREATE INDEX idx_scenario_rules_lookup ON scenario_rules(scenario_id, is_active, priority DESC);
CREATE INDEX idx_rule_suggestions_status ON rule_suggestions(status, confidence_score DESC);

-- Seed Initial Data (Migration from old scenario_configs)
-- We will keep scenario_configs for backward compatibility until the engine is fully swapped.
