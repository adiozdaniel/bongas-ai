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
            #[derive(sqlx::FromRow)]
            struct CategoryRow {
                slug: String,
            }

            let mut cats = vec![params.category.clone()];

            let subcats: Vec<CategoryRow> = sqlx::query_as(
                r#"
                SELECT slug FROM categories
                WHERE parent_slug = $1 OR slug = $1
                "#,
            )
            .bind(&params.category)
            .fetch_all(context.db_pool.as_ref())
            .await
            .unwrap_or_default();

            cats.extend(subcats.into_iter().map(|c| c.slug));
            cats
        } else {
            vec![params.category.clone()]
        };

        let order_clause = match params.sort_by.as_str() {
            "recent" => "release_date DESC NULLS LAST, added_date DESC NULLS LAST",
            "rating" => "COALESCE(user_rating, critic_rating, 0) DESC",
            "alphabetical" => "title ASC",
            _ => "popularity_score DESC NULLS LAST",
        };

        let mut query = format!(
            r#"
            SELECT i.item_id, i.title, i.popularity_score, i.user_rating, i.release_date, c.name as category_name
            FROM item_features i
            JOIN item_categories ic ON i.item_id = ic.item_id
            JOIN categories c ON ic.category_slug = c.slug
            WHERE ic.category_slug = ANY($1)
                AND i.is_active = true
            "#
        );

        if let Some(min_rating) = params.min_rating {
            query.push_str(&format!(
                " AND COALESCE(i.user_rating, i.critic_rating, 0) >= {}",
                min_rating
            ));
        }

        query.push_str(&format!(" ORDER BY {} LIMIT $2", order_clause));

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            title: Option<String>,
            popularity_score: Option<f32>,
            user_rating: Option<f32>,
            release_date: Option<chrono::DateTime<chrono::Utc>>,
            category_name: Option<String>,
        }

        let rows: Vec<Row> = sqlx::query_as(&query)
            .bind(&categories)
            .bind(params.limit as i64)
            .fetch_all(context.db_pool.as_ref())
            .await?;

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
                        "category_name": row.category_name,
                        "title": row.title,
                        "rating": row.user_rating,
                        "release_date": row.release_date.map(|d| d.to_rfc3339()),
                    }),
                }
            })
            .collect();

        Ok(items)
    }
}
