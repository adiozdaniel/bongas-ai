use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Limit number of items per preferred genre
    #[serde(default = "default_per_genre")]
    items_per_genre: usize,
    /// Total item limit
    #[serde(default = "default_limit")]
    limit: usize,
    /// Minimum affinity score to consider a genre preferred
    #[serde(default = "default_min_affinity")]
    min_affinity: f32,
    /// Include items from watchlist
    #[serde(default = "default_true")]
    include_watchlist: bool,
}

fn default_per_genre() -> usize {
    10
}

fn default_limit() -> usize {
    50
}

fn default_min_affinity() -> f32 {
    0.3
}

fn default_true() -> bool {
    true
}

pub struct FetchUserPreferencesStage;

#[async_trait]
impl PipelineStage for FetchUserPreferencesStage {
    fn name(&self) -> &str {
        "fetch_user_preferences"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let user_id = match context.user_id {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        // Get user's genre preferences
        #[derive(sqlx::FromRow)]
        struct UserPrefs {
            genre_affinity: Option<JsonValue>,
        }

        let prefs: Option<UserPrefs> = sqlx::query_as(
            "SELECT genre_affinity FROM user_features WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(context.db_pool.as_ref())
        .await?;

        let genre_affinity: Vec<(String, f32)> = prefs
            .and_then(|p| p.genre_affinity)
            .and_then(|v| serde_json::from_value::<std::collections::HashMap<String, f32>>(v).ok())
            .map(|m| {
                let mut vec: Vec<_> = m.into_iter()
                    .filter(|(_, score)| *score >= params.min_affinity)
                    .collect();
                vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                vec
            })
            .unwrap_or_default();

        if genre_affinity.is_empty() {
            return Ok(Vec::new());
        }

        let mut items: Vec<ScoredItem> = Vec::new();

        // Fetch items for each preferred genre
        for (genre, affinity) in genre_affinity.iter().take(5) {
            #[derive(sqlx::FromRow)]
            struct ItemRow {
                item_id: i32,
                title: Option<String>,
                popularity_score: Option<f32>,
            }

            let genre_items: Vec<ItemRow> = sqlx::query_as(
                r#"
                SELECT i.item_id, i.title, i.popularity_score
                FROM item_features i
                WHERE i.genres @> $1::jsonb
                    AND i.is_active = true
                ORDER BY i.popularity_score DESC NULLS LAST
                LIMIT $2
                "#,
            )
            .bind(json!([genre]))
            .bind(params.items_per_genre as i64)
            .fetch_all(context.db_pool.as_ref())
            .await?;

            for item in genre_items {
                let base_score = item.popularity_score.unwrap_or(0.5);
                items.push(ScoredItem {
                    item_id: item.item_id,
                    score: base_score * affinity,
                    metadata: json!({
                        "source": "user_preferences",
                        "matched_genre": genre,
                        "genre_affinity": affinity,
                        "title": item.title,
                    }),
                });
            }
        }

        // Optionally include watchlist items
        if params.include_watchlist {
            #[derive(sqlx::FromRow)]
            struct WatchlistRow {
                item_id: i32,
                added_at: chrono::DateTime<chrono::Utc>,
            }

            let watchlist: Vec<WatchlistRow> = sqlx::query_as(
                r#"
                SELECT item_id, added_at
                FROM user_watchlist
                WHERE user_id = $1
                ORDER BY added_at DESC
                LIMIT 20
                "#,
            )
            .bind(user_id)
            .fetch_all(context.db_pool.as_ref())
            .await
            .unwrap_or_default();

            for wl_item in watchlist {
                items.push(ScoredItem {
                    item_id: wl_item.item_id,
                    score: 1.0, // High score for watchlist items
                    metadata: json!({
                        "source": "watchlist",
                        "added_at": wl_item.added_at.to_rfc3339(),
                    }),
                });
            }
        }

        // Deduplicate and limit
        let mut seen = std::collections::HashSet::new();
        items.retain(|item| seen.insert(item.item_id));
        items.truncate(params.limit);

        Ok(items)
    }
}
