-- BONGAS-AI Finalized Database Schema
-- Phase 16: The Intelligent Brain (Consolidated)

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Create schema
CREATE SCHEMA IF NOT EXISTS bongas;
SET search_path TO bongas, public;

-- ============================================================================
-- 1. pipelines (The Strategy Logic)
-- ============================================================================
CREATE TABLE pipelines (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    definition JSONB NOT NULL,
    diversity_score FLOAT DEFAULT 0.5,
    coverage_impact VARCHAR(32) DEFAULT 'medium',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- 2. scenarios (The Product/Endpoints)
-- ============================================================================
CREATE TABLE scenarios (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT,
    target_kpi VARCHAR(32) DEFAULT 'retention',
    category VARCHAR(100),
    
    -- Configuration
    initial_display_limit INTEGER DEFAULT 5,
    scope JSONB DEFAULT '{}'::jsonb,
    cache_ttl_seconds INTEGER DEFAULT 300,
    use_l2_cache BOOLEAN DEFAULT true,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- 3. scenario_rules (The Routing Brain)
-- ============================================================================
CREATE TABLE scenario_rules (
    id SERIAL PRIMARY KEY,
    scenario_id INTEGER REFERENCES scenarios(id) ON DELETE CASCADE,
    pipeline_id INTEGER REFERENCES pipelines(id) ON DELETE RESTRICT,
    priority INTEGER DEFAULT 100,
    condition JSONB NOT NULL,
    is_active BOOLEAN DEFAULT true,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_scenario_rules_lookup ON scenario_rules(scenario_id, is_active, priority DESC);

-- ============================================================================
-- 4. rule_suggestions (The Suggestion Box)
-- ============================================================================
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

CREATE INDEX idx_rule_suggestions_status ON rule_suggestions(status, confidence_score DESC);
CREATE UNIQUE INDEX idx_rule_suggestions_dedup ON rule_suggestions(scenario_id, suggested_pipeline_id, md5(suggested_condition::text)) WHERE status = 'pending';

-- ============================================================================
-- 5. user_features (Feature Store)
-- ============================================================================
CREATE TABLE user_features (
    user_id INTEGER PRIMARY KEY,
    genre_affinity JSONB,
    total_watch_time_minutes INTEGER DEFAULT 0,
    total_videos_watched INTEGER DEFAULT 0,
    avg_completion_rate FLOAT DEFAULT 0.0,
    favorite_genres JSONB,
    watch_patterns JSONB,
    features_updated_at TIMESTAMP DEFAULT NOW(),
    last_interaction_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW()
);

-- ============================================================================
-- 6. item_features (Content Features)
-- ============================================================================
CREATE TABLE item_features (
    item_id INTEGER PRIMARY KEY,
    title VARCHAR(500),
    description TEXT,
    genres JSONB,
    tags JSONB,
    duration_seconds INTEGER,
    tfidf_vector JSONB,
    view_count INTEGER DEFAULT 0,
    like_count INTEGER DEFAULT 0,
    completion_rate FLOAT DEFAULT 0.0,
    trending_score FLOAT DEFAULT 0.0,
    is_active BOOLEAN DEFAULT true,
    published_at TIMESTAMP,
    features_updated_at TIMESTAMP DEFAULT NOW(),
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_item_genres ON item_features USING GIN(genres);
CREATE INDEX idx_item_trending ON item_features(trending_score DESC);

-- ============================================================================
-- 7. user_interactions (Event Log)
-- ============================================================================
CREATE TABLE user_interactions (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    interaction_type VARCHAR(50) NOT NULL,
    watch_duration_seconds INTEGER,
    completion_percentage FLOAT,
    implicit_rating FLOAT,
    explicit_rating INTEGER,
    scenario_slug VARCHAR(100),
    device_type VARCHAR(50),
    profile_id VARCHAR(100),
    context JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP DEFAULT NOW(),
    CONSTRAINT valid_interaction_type CHECK (
        interaction_type IN ('view', 'like', 'dislike', 'share', 'complete', 'skip', 'click', 'impression')
    )
);

CREATE INDEX idx_interactions_user ON user_interactions(user_id, created_at DESC);
CREATE INDEX idx_interactions_item ON user_interactions(item_id, created_at DESC);
CREATE INDEX idx_interactions_profile ON user_interactions(profile_id);

-- ============================================================================
-- 8. recommendation_cache_l2 (Staging Manager)
-- ============================================================================
CREATE TABLE recommendation_cache_l2 (
    id BIGSERIAL PRIMARY KEY,
    cache_key VARCHAR(500) UNIQUE NOT NULL,
    scenario_slug VARCHAR(100) NOT NULL,
    user_id INTEGER,
    context_hash VARCHAR(64),
    recommendations JSONB NOT NULL,
    cached_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    is_stale BOOLEAN DEFAULT false,
    staleness_reason VARCHAR(200),
    hit_count INTEGER DEFAULT 0,
    last_hit_at TIMESTAMP
);

-- ============================================================================
-- 9. model_registry (ML Model Versioning)
-- ============================================================================
CREATE TABLE model_registry (
    id SERIAL PRIMARY KEY,
    model_name VARCHAR(100) NOT NULL,
    version VARCHAR(50) NOT NULL,
    model_format VARCHAR(20) NOT NULL DEFAULT 'pytorch',
    file_path VARCHAR(500),
    onnx_model_path VARCHAR(500),
    s3_location VARCHAR(500),
    architecture_config JSONB,
    training_metrics JSONB,
    onnx_opset_version INTEGER,
    onnx_input_shapes JSONB,
    onnx_output_names JSONB,
    onnx_runtime_provider VARCHAR(50) DEFAULT 'cpu',
    status VARCHAR(50) DEFAULT 'training',
    deployed_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(model_name, version)
);

-- ============================================================================
-- 10. system_settings
-- ============================================================================
CREATE TABLE system_settings (
    key VARCHAR(100) PRIMARY KEY,
    value JSONB NOT NULL,
    description TEXT,
    updated_at TIMESTAMP DEFAULT NOW()
);

INSERT INTO system_settings (key, value, description) VALUES
('max_active_scenarios', '20'::jsonb, 'Maximum allowed scenarios with enabled=true');
