//! Phase 16: Strategy Resolver (The Macro-Brain)
//!
//! Evaluates scenario rules to select the most appropriate pipeline strategy
//! for a given request context. Supports high-priority overrides and 
//! contextual fallbacks.

use std::sync::Arc;
use serde_json::Value as JsonValue;
use tracing::{info, debug};
use arc_swap::ArcSwap;
use std::collections::HashMap;

use crate::pipeline::ExecutablePipeline;
use crate::pipeline::context::ExecutionContext;

/// Represents an active rule for strategy selection.
#[derive(Debug, Clone)]
pub struct ActiveRule {
    pub id: i32,
    pub priority: i32,
    pub condition: JsonValue,
    pub pipeline_slug: String,
    pub pipeline: Arc<ExecutablePipeline>,
}

/// In-memory cache of scenario rules for sub-microsecond resolution.
pub struct StrategyResolver {
    /// Map of scenario_slug -> List of rules (sorted by priority)
    rules: ArcSwap<HashMap<String, Vec<ActiveRule>>>,
}

impl StrategyResolver {
    pub fn new() -> Self {
        Self {
            rules: ArcSwap::from_pointee(HashMap::new()),
        }
    }

    /// Resolve the best pipeline for the current context.
    pub fn resolve(
        &self, 
        scenario_slug: &str, 
        context: &ExecutionContext
    ) -> Option<Arc<ExecutablePipeline>> {
        let all_rules = self.rules.load();
        
        if let Some(rules) = all_rules.get(scenario_slug) {
            for rule in rules {
                if self.evaluate_condition(&rule.condition, context) {
                    debug!(
                        scenario = %scenario_slug, 
                        rule_id = rule.id, 
                        strategy = %rule.pipeline_slug,
                        "Strategic rule matched"
                    );
                    return Some(rule.pipeline.clone());
                }
            }
        }
        
        None
    }

    /// Update the internal rule cache (called during reload).
    pub fn update_rules(&self, new_rules: HashMap<String, Vec<ActiveRule>>) {
        self.rules.store(Arc::new(new_rules));
        info!("Strategy Resolver rules updated (Atomic Swap)");
    }

    /// Evaluate a macro-routing condition against the context.
    fn evaluate_condition(&self, condition: &JsonValue, context: &ExecutionContext) -> bool {
        // Simple JSON-Logic-lite evaluation
        if let Some(obj) = condition.as_object() {
            for (key, expected_val) in obj {
                let actual_val = match key.as_str() {
                    "context.device_type" => context.device_type.as_deref().map(|s| serde_json::json!(s)),
                    "context.profile_id" => context.profile_id.as_deref().map(|s| serde_json::json!(s)),
                    "context.maturity_rating" => context.maturity_rating.as_deref().map(|s| serde_json::json!(s)),
                    "time.hour" => {
                        // In production, we'd pull this from a Chrono context helper
                        // For now, allow numeric comparison
                        None 
                    },
                    _ => None,
                }.unwrap_or(JsonValue::Null);

                if actual_val != *expected_val {
                    return false;
                }
            }
            return true;
        }
        
        // If condition is empty/null, it's a "Default" rule
        condition.is_null()
    }
}
