use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

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
                #[derive(sqlx::FromRow)]
                struct UserPrefs {
                    allow_explicit: Option<bool>,
                    allow_violence: Option<bool>,
                    allow_language: Option<bool>,
                    allow_drugs: Option<bool>,
                }
                if let Ok(Some(prefs)) = sqlx::query_as::<_, UserPrefs>(
                    "SELECT allow_explicit, allow_violence, allow_language, allow_drugs FROM user_content_preferences WHERE user_id = $1"
                )
                .bind(user_id)
                .fetch_optional(context.db_pool.as_ref())
                .await {
                    if let Some(v) = prefs.allow_explicit { params.allow_explicit = v; }
                    if let Some(v) = prefs.allow_violence { params.allow_violence = v; }
                    if let Some(v) = prefs.allow_language { params.allow_language = v; }
                    if let Some(v) = prefs.allow_drugs { params.allow_drugs = v; }
                }
            }
        }

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            is_explicit: Option<bool>,
            has_violence: Option<bool>,
            has_strong_language: Option<bool>,
            has_drug_content: Option<bool>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, is_explicit, has_violence, has_strong_language, has_drug_content
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let content_map: HashMap<i32, (bool, bool, bool, bool)> = rows
            .into_iter()
            .map(|row| {
                (
                    row.item_id,
                    (
                        row.is_explicit.unwrap_or(false),
                        row.has_violence.unwrap_or(false),
                        row.has_strong_language.unwrap_or(false),
                        row.has_drug_content.unwrap_or(false),
                    ),
                )
            })
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match content_map.get(&item.item_id) {
                    Some((is_explicit, has_violence, has_language, has_drugs)) => {
                        if *is_explicit && !params.allow_explicit {
                            return false;
                        }
                        if *has_violence && !params.allow_violence {
                            return false;
                        }
                        if *has_language && !params.allow_language {
                            return false;
                        }
                        if *has_drugs && !params.allow_drugs {
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
