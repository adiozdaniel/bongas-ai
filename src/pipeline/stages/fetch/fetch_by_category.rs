use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Category slug or ID to fetch content for
    category: String,
    /// Maximum items to fetch
    #[serde(default = "default_limit")]
    limit: usize,
    /// Sort by: "popularity", "recent", "rating", "alphabetical"
    #[serde(default = "default_sort")]
    sort_by: String,
    /// Include subcategories
    #[serde(default = "default_true")]
    include_subcategories: bool,
    /// Minimum rating filter
    #[serde(default)]
    min_rating: Option<f32>,
}

fn default_limit() -> usize {
    50
}

fn default_sort() -> String {
    "popularity".to_string()
}

fn default_true() -> bool {
    true
}

pub struct FetchByCategoryStage;

#[async_trait]
impl PipelineStage for FetchByCategoryStage {
    fn name(&self) -> &str {
        "fetch_by_category"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        // Get category and its subcategories if needed
        let categories: Vec<String> = if params.include_subcategories {
            let mut cats = vec![params.category.clone()];
            let subcats = context.item_feature_service
                .get_subcategories_for_category(&params.category)
                .await
                .unwrap_or_default();
            cats.extend(subcats);
            cats
        } else {
            vec![params.category.clone()]
        };

        let rows = context.item_feature_service.get_items_by_categories_advanced(
            &categories,
            params.min_rating,
            &params.sort_by,
            params.limit as i64,
        ).await?;

        let items: Vec<ScoredItem> = rows
            .into_iter()
            .map(|row| {
                let score = row.popularity_score.unwrap_or(0.5);

                ScoredItem {
                    item_id: row.item_id,
                    score,
                    metadata: json!({
                        "source": "category",
                        "category": params.category,
                        "title": row.title,
                        "rating": row.user_rating,
                        "release_date": row.release_date.map(|d: chrono::DateTime<chrono::Utc>| d.to_rfc3339()),
                    }),
                }
            })
            .collect();

        Ok(items)
    }
}
