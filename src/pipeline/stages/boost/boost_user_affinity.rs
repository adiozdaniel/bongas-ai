use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Maximum boost factor for perfect affinity match
    #[serde(default = "default_boost")]
    max_boost_factor: f32,
    /// Weight for genre affinity (0.0-1.0)
    #[serde(default = "default_genre_weight")]
    genre_weight: f32,
    /// Weight for creator/actor affinity (0.0-1.0)
    #[serde(default = "default_creator_weight")]
    creator_weight: f32,
    /// Weight for content type affinity (movies vs series)
    #[serde(default = "default_type_weight")]
    content_type_weight: f32,
}

fn default_boost() -> f32 {
    2.0
}

fn default_genre_weight() -> f32 {
    0.5
}

fn default_creator_weight() -> f32 {
    0.3
}

fn default_type_weight() -> f32 {
    0.2
}

pub struct BoostUserAffinityStage;

#[async_trait]
impl PipelineStage for BoostUserAffinityStage {
    fn name(&self) -> &str {
        "boost_user_affinity"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let user_id = match context.user_id {
            Some(id) => id,
            None => return Ok(input), // No user context, skip boosting
        };

        // Fetch user preferences
        #[derive(sqlx::FromRow)]
        struct UserPrefs {
            genre_affinity: Option<JsonValue>,
            favorite_creators: Option<JsonValue>,
            preferred_content_type: Option<String>,
        }

        let user_prefs: Option<UserPrefs> = sqlx::query_as(
            r#"
            SELECT genre_affinity, favorite_creators, preferred_content_type
            FROM user_features
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(context.db_pool.as_ref())
        .await?;

        let user_prefs = match user_prefs {
            Some(p) => p,
            None => return Ok(input), // No user preferences, skip boosting
        };

        let genre_affinity: HashMap<String, f32> = user_prefs.genre_affinity
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let favorite_creators: Vec<String> = user_prefs.favorite_creators
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let preferred_type = user_prefs.preferred_content_type.unwrap_or_default();

        // Fetch item features
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        #[derive(sqlx::FromRow)]
        struct ItemRow {
            item_id: i32,
            genres: Option<JsonValue>,
            creators: Option<JsonValue>,
            content_type: Option<String>,
        }

        let items: Vec<ItemRow> = sqlx::query_as(
            r#"
            SELECT item_id, genres, creators, content_type
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let item_map: HashMap<i32, (Vec<String>, Vec<String>, String)> = items
            .into_iter()
            .map(|row| {
                let genres: Vec<String> = row.genres
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                let creators: Vec<String> = row.creators
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                let content_type = row.content_type.unwrap_or_default();
                (row.item_id, (genres, creators, content_type))
            })
            .collect();

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                if let Some((genres, creators, content_type)) = item_map.get(&item.item_id) {
                    let mut affinity_score = 0.0;

                    // Genre affinity
                    let genre_score: f32 = genres
                        .iter()
                        .filter_map(|g| genre_affinity.get(&g.to_lowercase()))
                        .sum::<f32>()
                        / genres.len().max(1) as f32;
                    affinity_score += genre_score * params.genre_weight;

                    // Creator affinity
                    let creator_matches = creators
                        .iter()
                        .filter(|c| favorite_creators.iter().any(|fc| fc.to_lowercase() == c.to_lowercase()))
                        .count();
                    let creator_score = (creator_matches as f32 / favorite_creators.len().max(1) as f32).min(1.0);
                    affinity_score += creator_score * params.creator_weight;

                    // Content type affinity
                    let type_score = if content_type.to_lowercase() == preferred_type.to_lowercase() {
                        1.0
                    } else {
                        0.0
                    };
                    affinity_score += type_score * params.content_type_weight;

                    // Normalize and apply boost
                    let normalized_affinity = affinity_score / (params.genre_weight + params.creator_weight + params.content_type_weight);
                    let boost = 1.0 + (params.max_boost_factor - 1.0) * normalized_affinity;

                    item.score *= boost;
                    item.metadata["affinity_score"] = serde_json::json!(normalized_affinity);
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}
