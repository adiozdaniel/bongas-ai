use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Whether to allow explicit/mature content
    #[serde(default)]
    allow_explicit: bool,
    /// Whether to allow violent content
    #[serde(default)]
    allow_violence: bool,
    /// Whether to allow content with strong language
    #[serde(default = "default_true")]
    allow_language: bool,
    /// Whether to allow content with drug references
    #[serde(default = "default_true")]
    allow_drugs: bool,
    /// Use user's content preferences from profile
    #[serde(default)]
    use_user_preferences: bool,
}

fn default_true() -> bool {
    true
}

pub struct FilterExplicitContentStage;

#[async_trait]
impl PipelineStage for FilterExplicitContentStage {
    fn name(&self) -> &str {
        "filter_explicit_content"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let mut params: Params = serde_json::from_value(params.clone())?;

        // Optionally override with user preferences
        if params.use_user_preferences {
            if let Some(user_id) = context.user_id {
                if let Ok(Some(prefs)) = context.item_feature_service.get_user_content_preferences(user_id).await {
                    if let Some(v) = prefs.allow_explicit { params.allow_explicit = v; }
                    if let Some(v) = prefs.allow_violence { params.allow_violence = v; }
                    if let Some(v) = prefs.allow_language { params.allow_language = v; }
                    if let Some(v) = prefs.allow_drugs { params.allow_drugs = v; }
                }
            }
        }

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        if row.is_explicit.unwrap_or(false) && !params.allow_explicit {
                            return false;
                        }
                        if row.has_violence.unwrap_or(false) && !params.allow_violence {
                            return false;
                        }
                        if row.has_strong_language.unwrap_or(false) && !params.allow_language {
                            return false;
                        }
                        if row.has_drug_content.unwrap_or(false) && !params.allow_drugs {
                            return false;
                        }
                        true
                    }
                    None => true, // No content flags, include
                }
            })
            .collect();

        Ok(filtered)
    }
}
