use anyhow::{Result, anyhow};
use ndarray::Array2;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::TensorRef;
use std::path::Path;
use tracing::{info, debug};

pub struct OnnxInferenceEngine {
    session: Session,
    model_name: String,
    input_names: Vec<String>,
    _output_names: Vec<String>,
}

// Session is Send but not Sync by default in ort v2;
// we only call run(&mut self) from one thread at a time via Arc<RwLock>
unsafe impl Sync for OnnxInferenceEngine {}

impl OnnxInferenceEngine {
    /// Create new ONNX inference engine from model file
    pub fn new(model_path: impl AsRef<Path>, model_name: String) -> Result<Self> {
        let model_path = model_path.as_ref();

        info!("Loading ONNX model: {}", model_path.display());

        // Initialize ONNX Runtime session
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;

        // Extract input/output names
        let input_names: Vec<String> = session
            .inputs()
            .iter()
            .map(|input| input.name().to_string())
            .collect();

        let output_names: Vec<String> = session
            .outputs()
            .iter()
            .map(|output| output.name().to_string())
            .collect();

        info!(
            model_name = %model_name,
            inputs = ?input_names,
            outputs = ?output_names,
            "ONNX model loaded"
        );

        Ok(Self {
            session,
            model_name,
            input_names,
            _output_names: output_names,
        })
    }

    /// Run inference with user and item features (Two-Tower model)
    pub fn predict_two_tower(
        &mut self,
        user_features: Array2<f32>,
        item_features: Array2<f32>,
    ) -> Result<Vec<f32>> {
        debug!(
            model = %self.model_name,
            batch_size = user_features.nrows(),
            "Running ONNX two-tower inference"
        );

        // Create TensorRef views from ndarray
        let user_input = TensorRef::from_array_view(user_features.view())
            .map_err(|e| anyhow!("Failed to create user tensor: {}", e))?;
        let item_input = TensorRef::from_array_view(item_features.view())
            .map_err(|e| anyhow!("Failed to create item tensor: {}", e))?;

        // Run inference with named inputs
        let outputs = self.session.run(ort::inputs![
            self.input_names[0].clone() => user_input,
            self.input_names[1].clone() => item_input,
        ])
            .map_err(|e| anyhow!("ONNX inference failed: {}", e))?;

        // Extract scores from first output
        let (_shape, scores_slice) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("Failed to extract output tensor: {}", e))?;

        let scores_vec = scores_slice.to_vec();

        debug!(
            model = %self.model_name,
            score_count = scores_vec.len(),
            "ONNX inference complete"
        );

        Ok(scores_vec)
    }

    /// Batch inference for multiple user-item pairs
    pub fn predict_batch(
        &mut self,
        user_features: Vec<Vec<f32>>,
        item_features: Vec<Vec<f32>>,
    ) -> Result<Vec<f32>> {
        if user_features.len() != item_features.len() {
            return Err(anyhow!(
                "User and item feature counts must match: {} vs {}",
                user_features.len(),
                item_features.len()
            ));
        }

        let batch_size = user_features.len();
        if batch_size == 0 {
            return Ok(Vec::new());
        }

        // Convert Vec<Vec<f32>> to Array2<f32>
        let user_dim = user_features[0].len();
        let item_dim = item_features[0].len();

        let mut user_array = Array2::<f32>::zeros((batch_size, user_dim));
        let mut item_array = Array2::<f32>::zeros((batch_size, item_dim));

        for (i, (user, item)) in user_features.iter().zip(item_features.iter()).enumerate() {
            for (j, &val) in user.iter().enumerate() {
                user_array[[i, j]] = val;
            }
            for (j, &val) in item.iter().enumerate() {
                item_array[[i, j]] = val;
            }
        }

        self.predict_two_tower(user_array, item_array)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires actual ONNX model file
    fn test_onnx_inference() {
        let mut engine = OnnxInferenceEngine::new(
            "models/onnx/two_tower_v1.onnx",
            "two_tower_v1".to_string(),
        ).unwrap();

        let user_features = Array2::<f32>::zeros((2, 64));
        let item_features = Array2::<f32>::zeros((2, 32));

        let scores = engine.predict_two_tower(user_features, item_features).unwrap();
        assert_eq!(scores.len(), 2);
    }
}
