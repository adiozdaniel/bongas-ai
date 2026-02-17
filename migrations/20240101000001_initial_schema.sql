-- BONGAS-AI Initial Database Schema
-- Phase 1: Database Layer & JSONB Schema

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- For text search

-- Create schema
CREATE SCHEMA IF NOT EXISTS bongas;
SET search_path TO bongas, public;

-- ============================================================================
-- 1. scenario_configs (CORE DIFFERENTIATOR - JSONB Pipelines)
-- ============================================================================
CREATE TABLE scenario_configs (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(100) UNIQUE NOT NULL,
    name VARCHAR(200) NOT NULL,
    description TEXT,

    -- JSONB pipeline definition (zero hardcoded logic!)
    pipeline JSONB NOT NULL,

    -- Governance & Scoping (Phase 10)
    initial_display_limit INTEGER DEFAULT 5,
    scope JSONB DEFAULT '{}'::jsonb, -- {"regions": ["KE"], "content_types": ["video"]}

    -- Caching configuration
    cache_ttl_seconds INTEGER DEFAULT 300,
    use_l2_cache BOOLEAN DEFAULT true,
    staleness_rules JSONB,


    -- Status
    enabled BOOLEAN DEFAULT true,
    priority INTEGER DEFAULT 100,

    -- Metadata
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(100),
    version INTEGER DEFAULT 1,

    CONSTRAINT valid_slug CHECK (slug ~ '^[a-z0-9_-]+$')
);

CREATE INDEX idx_scenario_enabled ON scenario_configs(enabled, priority DESC);
CREATE INDEX idx_scenario_pipeline ON scenario_configs USING GIN(pipeline);

-- ============================================================================
-- 2. user_features (Feature Store)
-- ============================================================================
CREATE TABLE user_features (
    user_id INTEGER PRIMARY KEY,

    -- Feature vectors (computed by background workers)
    -- embedding VECTOR(128),  -- Enable if using pgvector extension
    genre_affinity JSONB,   -- {"action": 0.8, "comedy": 0.3, ...}

    -- Aggregated statistics (from ClickHouse)
    total_watch_time_minutes INTEGER DEFAULT 0,
    total_videos_watched INTEGER DEFAULT 0,
    avg_completion_rate FLOAT DEFAULT 0.0,
    favorite_genres JSONB,
    watch_patterns JSONB,

    -- Timestamps
    features_updated_at TIMESTAMP DEFAULT NOW(),
    last_interaction_at TIMESTAMP,

    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_user_features_updated ON user_features(features_updated_at);

-- ============================================================================
-- 3. item_features (Content Features)
-- ============================================================================
CREATE TABLE item_features (
    item_id INTEGER PRIMARY KEY,

    -- Content metadata
    title VARCHAR(500),
    description TEXT,
    genres JSONB,  -- ["action", "thriller"]
    tags JSONB,
    duration_seconds INTEGER,

    -- Feature vectors
    -- embedding VECTOR(128),  -- Enable if using pgvector extension
    tfidf_vector JSONB,

    -- Popularity metrics (from ClickHouse)
    view_count INTEGER DEFAULT 0,
    like_count INTEGER DEFAULT 0,
    completion_rate FLOAT DEFAULT 0.0,
    trending_score FLOAT DEFAULT 0.0,

    -- Timestamps
    published_at TIMESTAMP,
    features_updated_at TIMESTAMP DEFAULT NOW(),

    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_item_genres ON item_features USING GIN(genres);
CREATE INDEX idx_item_trending ON item_features(trending_score DESC);

-- ============================================================================
-- 4. user_interactions (Event Log)
-- ============================================================================
CREATE TABLE user_interactions (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,

    -- Interaction type
    interaction_type VARCHAR(50) NOT NULL,  -- 'view', 'like', 'dislike', 'share', 'complete'

    -- Implicit feedback
    watch_duration_seconds INTEGER,
    completion_percentage FLOAT,
    implicit_rating FLOAT,  -- Computed from watch percentage

    -- Explicit feedback
    explicit_rating INTEGER,  -- 1-5 stars

    -- Context
    scenario_slug VARCHAR(100),
    device_type VARCHAR(50),

    -- Timestamp
    created_at TIMESTAMP DEFAULT NOW(),

    CONSTRAINT valid_interaction_type CHECK (
        interaction_type IN ('view', 'like', 'dislike', 'share', 'complete', 'skip')
    )
);

CREATE INDEX idx_interactions_user ON user_interactions(user_id, created_at DESC);
CREATE INDEX idx_interactions_item ON user_interactions(item_id, created_at DESC);
CREATE INDEX idx_interactions_type ON user_interactions(interaction_type, created_at DESC);

-- ============================================================================
-- 5. recommendation_cache_l2 (Staging Manager - L2 Cache)
-- ============================================================================
CREATE TABLE recommendation_cache_l2 (
    id BIGSERIAL PRIMARY KEY,

    -- Cache key
    cache_key VARCHAR(500) UNIQUE NOT NULL,
    scenario_slug VARCHAR(100) NOT NULL,
    user_id INTEGER,
    context_hash VARCHAR(64),  -- SHA256 of context parameters

    -- Cached recommendations
    recommendations JSONB NOT NULL,  -- Array of item_ids with scores

    -- Staleness tracking
    cached_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    is_stale BOOLEAN DEFAULT false,
    staleness_reason VARCHAR(200),

    -- Metadata
    hit_count INTEGER DEFAULT 0,
    last_hit_at TIMESTAMP,

    CONSTRAINT valid_expiry CHECK (expires_at > cached_at)
);

CREATE INDEX idx_cache_l2_key ON recommendation_cache_l2(cache_key);
CREATE INDEX idx_cache_l2_expiry ON recommendation_cache_l2(expires_at) WHERE NOT is_stale;
CREATE INDEX idx_cache_l2_user ON recommendation_cache_l2(user_id, scenario_slug);

-- ============================================================================
-- 6. model_registry (ML Model Versioning with ONNX support)
-- ============================================================================
CREATE TABLE model_registry (
    id SERIAL PRIMARY KEY,

    -- Model identification
    model_name VARCHAR(100) NOT NULL,  -- 'two_tower', 'bert4rec', etc.
    version VARCHAR(50) NOT NULL,

    -- Model format and location
    model_format VARCHAR(20) NOT NULL DEFAULT 'pytorch',  -- 'pytorch', 'onnx', 'tensorflow'
    file_path VARCHAR(500),
    onnx_model_path VARCHAR(500),  -- Path to exported ONNX model
    s3_location VARCHAR(500),

    -- Model metadata
    architecture_config JSONB,
    training_metrics JSONB,

    -- ONNX-specific metadata
    onnx_opset_version INTEGER,
    onnx_input_shapes JSONB,
    onnx_output_names JSONB,
    onnx_runtime_provider VARCHAR(50) DEFAULT 'cpu',  -- 'cpu', 'cuda', 'tensorrt'

    -- Status
    status VARCHAR(50) DEFAULT 'training',  -- 'training', 'deployed', 'archived', 'exported_to_onnx'
    deployed_at TIMESTAMP,

    -- Timestamps
    created_at TIMESTAMP DEFAULT NOW(),

    UNIQUE(model_name, version),
    CONSTRAINT valid_status CHECK (
        status IN ('training', 'deployed', 'archived', 'failed', 'exported_to_onnx')
    ),
    CONSTRAINT valid_format CHECK (
        model_format IN ('pytorch', 'onnx', 'tensorflow')
    )
);

CREATE INDEX idx_model_deployed ON model_registry(model_name, status) WHERE status = 'deployed';
CREATE INDEX idx_model_onnx ON model_registry(model_format) WHERE model_format = 'onnx';

-- ============================================================================
-- 7. onnx_sessions (ONNX Runtime Session Management)
-- ============================================================================
CREATE TABLE onnx_sessions (
    id SERIAL PRIMARY KEY,

    -- Session identification
    session_id VARCHAR(100) UNIQUE NOT NULL,
    model_path VARCHAR(500) NOT NULL,

    -- Runtime configuration
    provider VARCHAR(50) DEFAULT 'cpu',  -- 'cpu', 'cuda', 'tensorrt'
    memory_pool_size_mb INTEGER DEFAULT 256,
    optimization_level VARCHAR(20) DEFAULT 'basic',  -- 'basic', 'extended', 'all'

    -- Performance metrics
    load_time_ms INTEGER,
    inference_latency_p50_ms INTEGER,
    inference_latency_p95_ms INTEGER,
    memory_usage_mb INTEGER,

    -- Status
    is_loaded BOOLEAN DEFAULT false,
    last_used_at TIMESTAMP DEFAULT NOW(),

    -- Metadata
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_onnx_sessions_model ON onnx_sessions(model_path);
CREATE INDEX idx_onnx_sessions_loaded ON onnx_sessions(is_loaded, last_used_at);

-- ============================================================================
-- 8. experiments (A/B Testing)
-- ============================================================================
CREATE TABLE experiments (
    id SERIAL PRIMARY KEY,

    -- Experiment details
    name VARCHAR(200) NOT NULL,
    description TEXT,
    hypothesis TEXT,

    -- Variants
    variants JSONB NOT NULL,  -- [{"id": "control", "weight": 0.5}, {"id": "treatment", "weight": 0.5}]

    -- Assignment
    assignment_method VARCHAR(50) DEFAULT 'random',  -- 'random', 'thompson_sampling', 'ucb'

    -- Status
    status VARCHAR(50) DEFAULT 'draft',  -- 'draft', 'running', 'completed', 'archived'
    started_at TIMESTAMP,
    ended_at TIMESTAMP,

    -- Results
    results JSONB,
    winner_variant_id VARCHAR(100),

    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(100),

    CONSTRAINT valid_status CHECK (
        status IN ('draft', 'running', 'completed', 'archived')
    )
);

-- ============================================================================
-- Triggers: auto-update updated_at columns
-- ============================================================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger to scenario_configs
CREATE TRIGGER update_scenario_configs_updated_at
    BEFORE UPDATE ON scenario_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to user_features
CREATE TRIGGER update_user_features_updated_at
    BEFORE UPDATE ON user_features
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to item_features
CREATE TRIGGER update_item_features_updated_at
    BEFORE UPDATE ON item_features
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to onnx_sessions
CREATE TRIGGER update_onnx_sessions_updated_at
    BEFORE UPDATE ON onnx_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 9. system_settings (Global Platform Governance)
-- ============================================================================
CREATE TABLE system_settings (
    key VARCHAR(100) PRIMARY KEY,
    value JSONB NOT NULL,
    description TEXT,
    updated_at TIMESTAMP DEFAULT NOW()
);

INSERT INTO system_settings (key, value, description) VALUES
('max_active_scenarios', '20'::jsonb, 'Maximum allowed scenarios with enabled=true');
