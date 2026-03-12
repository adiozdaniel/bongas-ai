//! Phase 1: Safety Simulator (The Guardian)
//! 
//! Provides shadow execution and validation for suggested strategies.
//! Prevents regressive or unstable changes from being promoted autonomously.

use std::collections::HashSet;
use anyhow::Result;
use crate::engine::coordination::service::RecommendationItem;

/// 🛡️ SafetySimulator: Ensures structural and behavioral stability of new strategies.
pub struct SafetySimulator;

impl Default for SafetySimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl SafetySimulator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate a confidence score (0.0 to 1.0) based on result drift.
    /// A score of 1.0 means the results are identical.
    pub fn calculate_drift_confidence(
        &self,
        control: &[RecommendationItem],
        suggested: &[RecommendationItem],
    ) -> f64 {
        if control.is_empty() && suggested.is_empty() {
            return 1.0;
        }
        if control.is_empty() || suggested.is_empty() {
            return 0.0;
        }

        // 1. Jaccard Similarity (Set Overlap)
        let control_ids: HashSet<i32> = control.iter().map(|i| i.item_id).collect();
        let suggested_ids: HashSet<i32> = suggested.iter().map(|i| i.item_id).collect();
        
        let intersection = control_ids.intersection(&suggested_ids).count() as f64;
        let union = control_ids.union(&suggested_ids).count() as f64;
        let set_similarity = intersection / union;

        // 2. Rank Stability (Order preservation for common items)
        let mut rank_diff_sum = 0.0;
        let mut common_count = 0;

        for (c_idx, c_item) in control.iter().enumerate() {
            if let Some(s_idx) = suggested.iter().position(|i| i.item_id == c_item.item_id) {
                // Weight rank difference by position (early items matter more)
                let weight = 1.0 / (c_idx as f64 + 1.0);
                let diff = (c_idx as f64 - s_idx as f64).abs();
                rank_diff_sum += diff * weight;
                common_count += 1;
            }
        }

        let rank_stability = if common_count > 0 {
            1.0 / (1.0 + (rank_diff_sum / common_count as f64))
        } else {
            0.0
        };

        // Blended Score: 60% Set Overlap, 40% Rank Stability
        (set_similarity * 0.6) + (rank_stability * 0.4)
    }

    /// Hardens the ML path by validating model output properties before promotion.
    /// This prevents "Memory Address Crashes" by detecting malformed or null-heavy tensors.
    pub fn validate_model_integrity(&self, items: &[RecommendationItem]) -> Result<bool> {
        if items.is_empty() {
            return Ok(true); // Empty is safe
        }

        for item in items {
            // Check for NaN or Infinite scores which often indicate FFI/ONNX instability
            if item.score.is_nan() || item.score.is_infinite() {
                return Ok(false);
            }
            
            // Validate metadata structure (placeholder for deeper tensor validation)
            if item.metadata.is_null() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
