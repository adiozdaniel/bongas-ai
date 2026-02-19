-- BONGAS-AI Phase 16: Interaction Context Extension
-- Adding profile_id and context to user_interactions for deep analytics.

SET search_path TO bongas, public;

ALTER TABLE user_interactions
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(100),
ADD COLUMN IF NOT EXISTS context JSONB DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_interactions_profile ON user_interactions(profile_id);
