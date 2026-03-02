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
CREATE TABLE IF NOT EXISTS pipelines (
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
CREATE TABLE IF NOT EXISTS scenarios (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT,
    target_kpi VARCHAR(32) DEFAULT 'retention',
    category VARCHAR(100),
    
    -- Safety & Targeting Defaults (KFCB Standard)
    maturity_rating VARCHAR(32) DEFAULT '18', -- Global safety ceiling
    
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
CREATE TABLE IF NOT EXISTS scenario_rules (
    id SERIAL PRIMARY KEY,
    scenario_id INTEGER REFERENCES scenarios(id) ON DELETE CASCADE,
    pipeline_id INTEGER REFERENCES pipelines(id) ON DELETE RESTRICT,
    
    -- Contextual Routing
    device_type VARCHAR(32) DEFAULT 'default',
    maturity_rating VARCHAR(32) DEFAULT 'all',
    
    priority INTEGER DEFAULT 100,
    condition JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active BOOLEAN DEFAULT true,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scenario_rules_lookup ON scenario_rules(scenario_id, device_type, maturity_rating, is_active, priority DESC);

-- ============================================================================
-- 4. rule_suggestions (The Suggestion Box)
-- ============================================================================
CREATE TABLE IF NOT EXISTS rule_suggestions (
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

CREATE INDEX IF NOT EXISTS idx_rule_suggestions_status ON rule_suggestions(status, confidence_score DESC);

-- ============================================================================
-- 5. user_features (Feature Store)
-- ============================================================================
CREATE TABLE IF NOT EXISTS user_features (
    user_id INTEGER PRIMARY KEY,
    genre_affinity JSONB,
    disliked_genres JSONB,
    total_watch_time_minutes INTEGER DEFAULT 0,
    total_videos_watched INTEGER DEFAULT 0,
    avg_completion_rate FLOAT DEFAULT 0.0,
    favorite_genres JSONB,
    favorite_creators JSONB,
    watch_patterns JSONB,
    preferred_content_type VARCHAR(50),
    embedding FLOAT4[], -- pgvector VECTOR(128) if enabled
    features_updated_at TIMESTAMP DEFAULT NOW(),
    last_interaction_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW()
);

-- ============================================================================
-- 6. item_features (Content Features)
-- ============================================================================
CREATE TABLE IF NOT EXISTS item_features (
    item_id INTEGER PRIMARY KEY,
    title VARCHAR(500),
    description TEXT,
    genres JSONB,
    tags JSONB,
    creators JSONB,
    directors JSONB,
    studios JSONB,
    actors JSONB,
    content_type VARCHAR(50),
    language VARCHAR(50),
    audio_languages JSONB,
    subtitle_languages JSONB,
    age_rating VARCHAR(20),
    duration_seconds INTEGER,
    release_year INTEGER,
    release_date TIMESTAMP,
    published_at TIMESTAMP,
    added_date TIMESTAMP,
    available_from TIMESTAMP,
    available_until TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    
    -- Quality & Technical
    max_resolution VARCHAR(20),
    has_hdr BOOLEAN,
    has_dolby_vision BOOLEAN,
    has_dolby_atmos BOOLEAN,
    
    -- Content Warnings
    is_explicit BOOLEAN,
    has_violence BOOLEAN,
    has_strong_language BOOLEAN,
    has_drug_content BOOLEAN,
    
    -- Country availability
    available_countries JSONB,
    blocked_countries JSONB,
    
    -- Specialized Tags
    seasonal_tags JSONB,
    holiday_tags JSONB,
    themes JSONB,
    is_award_winner BOOLEAN,
    required_tier VARCHAR(50),
    is_free BOOLEAN,
    
    -- Scores
    view_count INTEGER DEFAULT 0,
    like_count INTEGER DEFAULT 0,
    comment_count BIGINT DEFAULT 0,
    share_count BIGINT DEFAULT 0,
    save_count BIGINT DEFAULT 0,
    completion_rate FLOAT DEFAULT 0.0,
    trending_score FLOAT DEFAULT 0.0,
    popularity_score FLOAT DEFAULT 0.0,
    user_rating FLOAT,
    user_rating_count INTEGER,
    critic_rating FLOAT,
    critic_rating_count INTEGER,
    
    -- Embeddings & vectors
    embedding FLOAT4[],
    tfidf_vector JSONB,
    
    features_updated_at TIMESTAMP DEFAULT NOW(),
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_item_genres ON item_features USING GIN(genres);
CREATE INDEX IF NOT EXISTS idx_item_trending ON item_features(trending_score DESC);

-- ============================================================================
-- 7. user_interactions (Event Log)
-- ============================================================================
CREATE TABLE IF NOT EXISTS user_interactions (
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

CREATE INDEX IF NOT EXISTS idx_interactions_user ON user_interactions(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_interactions_item ON user_interactions(item_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_interactions_profile ON user_interactions(profile_id);

-- ============================================================================
-- 7a. user_arrival_patterns (Predictive Warmer)
-- ============================================================================
CREATE TABLE IF NOT EXISTS user_arrival_patterns (
    user_id INTEGER PRIMARY KEY,
    hour_mask BIGINT DEFAULT 0,
    last_active_at TIMESTAMP DEFAULT NOW()
);

-- ============================================================================
-- 8. recommendation_cache_l2 (Staging Manager)
-- ============================================================================
CREATE TABLE IF NOT EXISTS recommendation_cache_l2 (
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
CREATE TABLE IF NOT EXISTS model_registry (
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
CREATE TABLE IF NOT EXISTS system_settings (
    key VARCHAR(100) PRIMARY KEY,
    value JSONB NOT NULL,
    description TEXT,
    updated_at TIMESTAMP DEFAULT NOW()
);

INSERT INTO system_settings (key, value, description) 
VALUES ('max_active_scenarios', '20'::jsonb, 'Maximum allowed scenarios with enabled=true')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- 11. page_layouts (Dynamic UI Layouts - SDUI)
-- ============================================================================
CREATE TABLE IF NOT EXISTS page_layouts (
    id SERIAL PRIMARY KEY,
    page_slug VARCHAR(64) NOT NULL,
    device_type VARCHAR(32) DEFAULT 'default',
    maturity_rating VARCHAR(32) DEFAULT 'all',
    priority INTEGER DEFAULT 0,
    composition JSONB NOT NULL, -- Array of objects: [{"slug": "...", "row_type": "..."}]
    
    is_active BOOLEAN DEFAULT true,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(page_slug, device_type, maturity_rating)
);

CREATE INDEX IF NOT EXISTS idx_page_layouts_resolver ON page_layouts (page_slug, device_type, maturity_rating) 
WHERE is_active = true AND is_deleted = false;
