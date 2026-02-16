//! Pipeline Optimizer for Phase 6 Hardware-Level Perfection.
//!
//! # Architecture
//! ```text
//! ExecutablePipeline ───> [Optimizer] ───> FusedPipeline
//!                                            │
//!                                            └─> [Stage Fusion]: Combine Boosters
//!                                            └─> [Kernelization]: Vectorized Scoring
//! ```
//!
//! This service transforms a standard linked pipeline into an optimized version
//! by merging adjacent mathematical stages to eliminate redundant iterations.

use crate::pipeline::{ExecutionNode, BoundStage};

pub struct PipelineOptimizer;

impl PipelineOptimizer {
    /// Optimize a sequence of execution nodes by fusing adjacent compatible stages.
    pub fn optimize(nodes: Vec<ExecutionNode>) -> Vec<ExecutionNode> {
        if nodes.is_empty() {
            return nodes;
        }

        let mut optimized = Vec::with_capacity(nodes.len());
        let mut current_fusion_batch: Vec<BoundStage> = Vec::new();

        for node in nodes {
            match node {
                ExecutionNode::Single(bound) => {
                    // Fusion Rule: Adjacent "boost_" stages that don't do I/O 
                    // (Note: in real production we'd check a flag like .is_io_bound())
                    if bound.stage_type.starts_with("boost_") {
                        current_fusion_batch.push(bound);
                    } else {
                        if !current_fusion_batch.is_empty() {
                            optimized.push(Self::create_fused_node(std::mem::take(&mut current_fusion_batch)));
                        }
                        optimized.push(ExecutionNode::Single(bound));
                    }
                }
                ExecutionNode::Parallel(stages) => {
                    if !current_fusion_batch.is_empty() {
                        optimized.push(Self::create_fused_node(std::mem::take(&mut current_fusion_batch)));
                    }
                    optimized.push(ExecutionNode::Parallel(stages));
                }
                ExecutionNode::Fused(stages) => {
                    if !current_fusion_batch.is_empty() {
                        optimized.push(Self::create_fused_node(std::mem::take(&mut current_fusion_batch)));
                    }
                    optimized.push(ExecutionNode::Fused(stages));
                }
            }
        }

        if !current_fusion_batch.is_empty() {
            optimized.push(Self::create_fused_node(current_fusion_batch));
        }

        optimized
    }

    fn create_fused_node(stages: Vec<BoundStage>) -> ExecutionNode {
        if stages.len() == 1 {
            ExecutionNode::Single(stages.into_iter().next().unwrap())
        } else {
            // JIT-lite: Group these stages for a single-pass execution in the executor
            ExecutionNode::Fused(stages)
        }
    }
}
