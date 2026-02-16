use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;

use crate::pipeline::{PipelineStage, StageDataKind, PipelineError, ExecutablePipeline};
use crate::db::models::{PipelineDefinition, PipelineStageConfig};

/// Validates the structural integrity and type safety of a pipeline.
pub struct PipelineValidator {
    stage_registry: HashMap<String, Arc<dyn PipelineStage>>,
}

impl PipelineValidator {
    pub fn new(stage_registry: HashMap<String, Arc<dyn PipelineStage>>) -> Self {
        Self { stage_registry }
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
            let stage_impl = self.stage_registry.get(&config.r#type)
                .ok_or_else(|| anyhow::anyhow!("Stage type '{}' not found in registry", config.r#type))?;

            let input_req = stage_impl.input_type();
            let output_prod = stage_impl.output_type();

            // Validate that current output can feed into this stage's input
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

    /// Validate an already-linked executable pipeline.
    pub fn validate_executable(&self, pipeline: &ExecutablePipeline) -> Result<()> {
        let mut current_output = StageDataKind::Empty;

        for (idx, node) in pipeline.nodes.iter().enumerate() {
            match node {
                crate::pipeline::ExecutionNode::Single(stage) => {
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
                crate::pipeline::ExecutionNode::Parallel(stages) => {
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
                                return Err(anyhow::anyhow!("Parallel stages in node {} produce inconsistent output types", idx));
                            }
                        } else {
                            first_output = Some(output_prod);
                        }
                    }
                    current_output = first_output.unwrap_or(StageDataKind::Empty);
                }
            }
        }

        Ok(())
    }

    /// Helper to check if two data kinds are compatible.
    fn are_types_compatible(&self, output: StageDataKind, input: StageDataKind) -> bool {
        // Empty output can only go into stages that accept Empty input
        // ScoredItems can go into ScoredItems
        // ItemIds can go into ItemIds (or we might allow ItemIds -> ScoredItems if stage handles it)
        output == input || (output == StageDataKind::Empty && input == StageDataKind::Empty)
    }
}
