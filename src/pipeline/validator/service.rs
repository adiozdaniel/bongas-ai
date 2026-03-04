//! Pipeline structural validator for the "Safety Gate" activation phase.

use anyhow::Result;
use std::collections::HashMap;

use crate::pipeline::{StageDataKind, PipelineError, ExecutablePipeline, ExecutionNode};
use crate::pipeline::registry::service::PipelineRegistry;
use crate::db::{PipelineDefinition, PipelineStageConfig};

/// Validates the structural integrity and type safety of a pipeline.
pub struct PipelineValidator {
    registry: PipelineRegistry,
}

impl PipelineValidator {
    /// Create a new validator with the provided stage registry.
    pub fn new(registry: PipelineRegistry) -> Self {
        Self { registry }
    }

    /// Validate a raw pipeline definition from the database.
    pub fn validate_definition(&self, definition: &PipelineDefinition) -> Result<()> {
        self.validate_stage_sequence(&definition.stages)?;
        
        if let Some(ref fallback) = definition.fallback_stages {
            self.validate_stage_sequence(fallback)?;
        }
        
        Ok(())
    }

    /// Validate a sequence of stage configurations.
    fn validate_stage_sequence(&self, stages: &[PipelineStageConfig]) -> Result<()> {
        if stages.is_empty() {
            return Ok(());
        }

        let mut current_output = StageDataKind::Empty;

        for (idx, config) in stages.iter().enumerate() {
            match config.r#type.as_str() {
                "branch" => {
                    let if_true: Vec<PipelineStageConfig> = serde_json::from_value(
                        config.params.get("if_true").cloned().unwrap_or_default()
                    )?;
                    let if_false: Vec<PipelineStageConfig> = serde_json::from_value(
                        config.params.get("if_false").cloned().unwrap_or_default()
                    )?;
                    
                    self.validate_stage_sequence_with_input(&if_true, current_output)?;
                    self.validate_stage_sequence_with_input(&if_false, current_output)?;
                    
                    current_output = StageDataKind::ScoredItems;
                    continue;
                }
                "ensemble" => {
                    let source_configs: Vec<serde_json::Value> = serde_json::from_value(
                        config.params.get("sources").cloned().unwrap_or_default()
                    )?;
                    for sc in source_configs {
                        let inner_stages: Vec<PipelineStageConfig> = serde_json::from_value(
                            sc.get("stages").cloned().unwrap_or_default()
                        )?;
                        self.validate_stage_sequence_with_input(&inner_stages, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                    continue;
                }
                "interleave" => {
                    let source_map: HashMap<String, Vec<PipelineStageConfig>> = serde_json::from_value(
                        config.params.get("sources").cloned().unwrap_or_default()
                    )?;
                    for inner_stages in source_map.values() {
                        self.validate_stage_sequence_with_input(inner_stages, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                    continue;
                }
                _ => {}
            }

            let stage_impl = self.registry.get(&config.r#type)
                .ok_or_else(|| anyhow::anyhow!("Stage type '{}' not found in registry", config.r#type))?;

            let input_req = stage_impl.input_type();
            let output_prod = stage_impl.output_type();

            if !self.are_types_compatible(current_output, input_req) {
                return Err(PipelineError::TypeMismatch {
                    stage_index: if idx > 0 { idx - 1 } else { 0 },
                    stage_type: if idx > 0 { stages[idx-1].r#type.clone() } else { "Start".to_string() },
                    output: current_output,
                    next_stage_index: idx,
                    next_stage_type: config.r#type.clone(),
                    input: input_req,
                }.into());
            }

            current_output = output_prod;
        }

        Ok(())
    }

    /// Helper to validate a sequence with a specific starting input type.
    fn validate_stage_sequence_with_input(&self, stages: &[PipelineStageConfig], input_type: StageDataKind) -> Result<()> {
        if stages.is_empty() {
            return Ok(());
        }

        let mut current_output = input_type;

        for (idx, config) in stages.iter().enumerate() {
            match config.r#type.as_str() {
                "branch" => {
                    let if_true: Vec<PipelineStageConfig> = serde_json::from_value(
                        config.params.get("if_true").cloned().unwrap_or_default()
                    )?;
                    let if_false: Vec<PipelineStageConfig> = serde_json::from_value(
                        config.params.get("if_false").cloned().unwrap_or_default()
                    )?;
                    
                    self.validate_stage_sequence_with_input(&if_true, current_output)?;
                    self.validate_stage_sequence_with_input(&if_false, current_output)?;
                    
                    current_output = StageDataKind::ScoredItems;
                    continue;
                }
                "ensemble" => {
                    let source_configs: Vec<serde_json::Value> = serde_json::from_value(
                        config.params.get("sources").cloned().unwrap_or_default()
                    )?;
                    for sc in source_configs {
                        let inner_stages: Vec<PipelineStageConfig> = serde_json::from_value(
                            sc.get("stages").cloned().unwrap_or_default()
                        )?;
                        self.validate_stage_sequence_with_input(&inner_stages, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                    continue;
                }
                "interleave" => {
                    let source_map: HashMap<String, Vec<PipelineStageConfig>> = serde_json::from_value(
                        config.params.get("sources").cloned().unwrap_or_default()
                    )?;
                    for inner_stages in source_map.values() {
                        self.validate_stage_sequence_with_input(inner_stages, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                    continue;
                }
                _ => {}
            }

            let stage_impl = self.registry.get(&config.r#type)
                .ok_or_else(|| anyhow::anyhow!("Stage type '{}' not found in registry", config.r#type))?;

            if !self.are_types_compatible(current_output, stage_impl.input_type()) {
                return Err(anyhow::anyhow!("Type mismatch at stage {}: {:?}", idx, config.r#type));
            }
            current_output = stage_impl.output_type();
        }
        Ok(())
    }

    /// Validate an already-linked executable pipeline.
    pub fn validate_executable(&self, pipeline: &ExecutablePipeline) -> Result<()> {
        let mut current_output = StageDataKind::Empty;

        for (idx, node) in pipeline.nodes.iter().enumerate() {
            match node {
                ExecutionNode::Single(stage) => {
                    let input_req = stage.implementation.input_type();
                    let output_prod = stage.implementation.output_type();

                    if !self.are_types_compatible(current_output, input_req) {
                        return Err(PipelineError::TypeMismatch {
                            stage_index: if idx > 0 { idx - 1 } else { 0 },
                            stage_type: "Previous Node".to_string(),
                            output: current_output,
                            next_stage_index: idx,
                            next_stage_type: stage.stage_type.clone(),
                            input: input_req,
                        }.into());
                    }
                    current_output = output_prod;
                }
                ExecutionNode::Parallel { stages, .. } => {
                    let mut first_output = None;
                    for stage in stages {
                        let input_req = stage.implementation.input_type();
                        let output_prod = stage.implementation.output_type();

                        if !self.are_types_compatible(current_output, input_req) {
                            return Err(PipelineError::TypeMismatch {
                                stage_index: if idx > 0 { idx - 1 } else { 0 },
                                stage_type: "Previous Node".to_string(),
                                output: current_output,
                                next_stage_index: idx,
                                next_stage_type: stage.stage_type.clone(),
                                input: input_req,
                            }.into());
                        }

                        if let Some(prev_out) = first_output {
                            if prev_out != output_prod {
                                return Err(anyhow::anyhow!("Parallel stages produce inconsistent output types"));
                            }
                        } else {
                            first_output = Some(output_prod);
                        }
                    }
                    current_output = first_output.unwrap_or(StageDataKind::Empty);
                }
                ExecutionNode::Fused(stages) => {
                    for stage in stages {
                        let input_req = stage.implementation.input_type();
                        let output_prod = stage.implementation.output_type();

                        if !self.are_types_compatible(current_output, input_req) {
                            return Err(PipelineError::TypeMismatch {
                                stage_index: if idx > 0 { idx - 1 } else { 0 },
                                stage_type: "Previous Fused Stage".to_string(),
                                output: current_output,
                                next_stage_index: idx,
                                next_stage_type: stage.stage_type.clone(),
                                input: input_req,
                            }.into());
                        }
                        current_output = output_prod;
                    }
                }
                ExecutionNode::Branch { if_true, if_false, .. } => {
                    self.validate_nodes_internal(if_true, current_output)?;
                    self.validate_nodes_internal(if_false, current_output)?;
                    current_output = StageDataKind::ScoredItems;
                }
                ExecutionNode::Ensemble { sources, .. } => {
                    for source in sources {
                        self.validate_nodes_internal(&source.nodes, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                }
                ExecutionNode::Interleave { sources, .. } => {
                    for nodes in sources.values() {
                        self.validate_nodes_internal(nodes, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                }
            }
        }

        Ok(())
    }

    fn validate_nodes_internal(&self, nodes: &[ExecutionNode], mut current_output: StageDataKind) -> Result<StageDataKind> {
        for node in nodes {
            match node {
                ExecutionNode::Single(stage) => {
                    let input_req = stage.implementation.input_type();
                    let output_prod = stage.implementation.output_type();
                    if !self.are_types_compatible(current_output, input_req) {
                        return Err(anyhow::anyhow!("Type mismatch in structural node"));
                    }
                    current_output = output_prod;
                }
                ExecutionNode::Parallel { stages, .. } => {
                    for stage in stages {
                        if !self.are_types_compatible(current_output, stage.implementation.input_type()) {
                            return Err(anyhow::anyhow!("Type mismatch in structural parallel node"));
                        }
                    }
                    current_output = StageDataKind::ScoredItems;
                }
                ExecutionNode::Fused(stages) => {
                    for stage in stages {
                        if !self.are_types_compatible(current_output, stage.implementation.input_type()) {
                            return Err(anyhow::anyhow!("Type mismatch in structural fused node"));
                        }
                        current_output = stage.implementation.output_type();
                    }
                }
                ExecutionNode::Branch { if_true, if_false, .. } => {
                    self.validate_nodes_internal(if_true, current_output)?;
                    self.validate_nodes_internal(if_false, current_output)?;
                    current_output = StageDataKind::ScoredItems;
                }
                ExecutionNode::Ensemble { sources, .. } => {
                    for source in sources {
                        self.validate_nodes_internal(&source.nodes, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                }
                ExecutionNode::Interleave { sources, .. } => {
                    for nodes in sources.values() {
                        self.validate_nodes_internal(nodes, current_output)?;
                    }
                    current_output = StageDataKind::ScoredItems;
                }
            }
        }
        Ok(current_output)
    }

    fn are_types_compatible(&self, output: StageDataKind, input: StageDataKind) -> bool {
        if input == StageDataKind::Empty {
            return true;
        }
        output == input
    }
}
