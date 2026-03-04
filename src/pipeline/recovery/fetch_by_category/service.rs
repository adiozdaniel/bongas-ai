use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind, CompactMetadata};
use crate::pipeline::context::service::ExecutionContext;

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

    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

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

                let mut item = ScoredItem::new(
                    row.item_id,
                    score,
                    json!({
                        "source": "category",
                        "category": params.category,
                        "title": row.title,
                        "rating": row.user_rating,
                        "age_rating": row.age_rating,
                        "published_at": row.published_at.or(row.release_date).map(|d| d.to_rfc3339()),
                        "genres": row.genres,
                        "release_date": row.release_date.map(|d: chrono::DateTime<chrono::Utc>| d.to_rfc3339()),
                    }),
                );

                // Phase 6: Populate Zero-Copy Fast Metadata
                let compact = CompactMetadata {
                    features: vec![score, row.user_rating.unwrap_or(0.0)],
                    flags: 0,
                    category_id: 0,
                };
                if let Ok(bytes) = rkyv::to_bytes::<_, 256>(&compact) {
                    item.fast_metadata = Some(bytes.to_vec());
                }

                item
            })
            .collect();

        Ok(items)
    }
}
