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
use chrono::Timelike;

use crate::pipeline::types::models::ExecutablePipeline;
use crate::pipeline::context::service::ExecutionContext;
use crate::db::PipelineDefinition;

/// Represents an active rule for strategy selection.
#[derive(Debug, Clone)]
pub struct ActiveRule {
    pub id: i32,
    pub priority: i32,
    
    // Contextual Targeting
    pub device_type: Option<String>,
    pub maturity_rating: Option<String>,
    
    pub condition: JsonValue,
    pub pipeline_slug: String,
    pub pipeline_definition: PipelineDefinition,
    pub pipeline: Option<Arc<ExecutablePipeline>>,
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
                if self.evaluate_rule(rule, context) {
                    if let Some(ref pipeline) = rule.pipeline {
                        debug!(
                            scenario = %scenario_slug, 
                            rule_id = rule.id, 
                            strategy = %rule.pipeline_slug,
                            device = ?rule.device_type,
                            maturity = ?rule.maturity_rating,
                            "Strategic rule matched"
                        );
                        return Some(pipeline.clone());
                    }
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

    /// Full evaluation of an ActiveRule against the context.
    fn evaluate_rule(&self, rule: &ActiveRule, context: &ExecutionContext) -> bool {
        // 1. Check First-Class Contextual Columns (Targeting Resolver)
        
        // Device Type Match (supports 'default' or NULL as catch-all)
        if let Some(ref rule_device) = rule.device_type {
            if rule_device != "default" && context.device_type.as_ref() != Some(rule_device) {
                return false;
            }
        }

        // Maturity Rating Match (supports 'all' or NULL as catch-all)
        if let Some(ref rule_maturity) = rule.maturity_rating {
            if rule_maturity != "all" && context.maturity_rating.as_ref() != Some(rule_maturity) {
                return false;
            }
        }

        // 2. Evaluate Dynamic JSON Condition
        self.evaluate_condition(&rule.condition, context)
    }

    /// Evaluate a macro-routing condition against the context.
    fn evaluate_condition(&self, condition: &JsonValue, context: &ExecutionContext) -> bool {
        // Simple JSON-Logic-lite evaluation
        if let Some(obj) = condition.as_object() {
            for (key, expected_val) in obj {
                match key.as_str() {
                    "context.device_type" => {
                        if context.device_type.as_deref().map(|s| serde_json::json!(s)).unwrap_or(JsonValue::Null) != *expected_val {
                            return false;
                        }
                    },
                    "context.profile_id" => {
                        if context.profile_id.as_deref().map(|s| serde_json::json!(s)).unwrap_or(JsonValue::Null) != *expected_val {
                            return false;
                        }
                    },
                    "context.maturity_rating" => {
                        if context.maturity_rating.as_deref().map(|s| serde_json::json!(s)).unwrap_or(JsonValue::Null) != *expected_val {
                            return false;
                        }
                    },
                    "time.hour" => {
                        let actual_hour = context.request_time.hour();
                        if serde_json::json!(actual_hour) != *expected_val {
                            return false;
                        }
                    },
                    "time.hour_range" => {
                        if let Some(range) = expected_val.as_array() {
                            if range.len() == 2 {
                                let start = range[0].as_u64().unwrap_or(0) as u32;
                                let end = range[1].as_u64().unwrap_or(23) as u32;
                                let actual_hour = context.request_time.hour();
                                
                                // Handle wrapping ranges (e.g., [22, 4] for 10 PM to 4 AM)
                                let in_range = if start <= end {
                                    actual_hour >= start && actual_hour <= end
                                } else {
                                    actual_hour >= start || actual_hour <= end
                                };
                                
                                if !in_range {
                                    return false;
                                }
                            }
                        }
                    },
                    _ => {
                        debug!(key = %key, "Unknown condition key encountered in strategic rule; skipping rule match.");
                        return false;
                    }
                }
            }
            return true;
        }
        
        // If condition is empty/null, it's a "Default" rule
        condition.is_null() || (condition.is_object() && condition.as_object().unwrap().is_empty())
    }
}
