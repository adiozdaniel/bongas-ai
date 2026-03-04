use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// List of language codes to filter by (e.g., ["en", "es", "fr"])
    languages: Vec<String>,
    /// "include" = keep only these languages, "exclude" = remove these languages
    #[serde(default = "default_mode")]
    mode: String,
    /// Whether to include items with unknown language
    #[serde(default)]
    include_unknown: bool,
}

fn default_mode() -> String {
    "include".to_string()
}

pub struct FilterByLanguageStage;

#[async_trait]
impl PipelineStage for FilterByLanguageStage {
    fn name(&self) -> &str {
        "filter_by_language"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let target_languages: Vec<String> = params.languages.iter().map(|l| l.to_lowercase()).collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        let mut langs = Vec::new();
                        if let Some(ref lang) = row.language {
                            langs.push(lang.to_lowercase());
                        }
                        if let Some(ref audio) = row.audio_languages {
                            if let Ok(audio_langs) = serde_json::from_value::<Vec<String>>(audio.clone()) {
                                langs.extend(audio_langs.into_iter().map(|l| l.to_lowercase()));
                            }
                        }
                        if let Some(ref subs) = row.subtitle_languages {
                            if let Ok(sub_langs) = serde_json::from_value::<Vec<String>>(subs.clone()) {
                                langs.extend(sub_langs.into_iter().map(|l| l.to_lowercase()));
                            }
                        }

                        if langs.is_empty() {
                            return params.include_unknown;
                        }

                        let has_language = langs.iter().any(|l| target_languages.contains(l));
                        match params.mode.as_str() {
                            "include" => has_language,
                            "exclude" => !has_language,
                            _ => true,
                        }
                    }
                    None => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
