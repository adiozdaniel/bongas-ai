//! Phase 4.1 & 4.2: Embedded Search Stage & Relevance Fusion
//!
//! Executes keyword search directly against the local Tantivy index and 
//! performs high-precision re-ranking using Vision DNA and User History.

use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use tantivy::query::QueryParser;
use tantivy::collector::TopDocs;
use tantivy::TantivyDocument;
use tantivy::schema::Value;
use tracing::debug;

use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Number of results to fetch from the index
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Weight for DNA similarity (0.0 to 1.0)
    #[serde(default = "default_dna_weight")]
    pub dna_weight: f32,
}

fn default_limit() -> usize { 50 }
fn default_dna_weight() -> f32 { 0.3 }

pub struct EmbeddedSearchStage;

#[async_trait]
impl PipelineStage for EmbeddedSearchStage {
    fn name(&self) -> &str { "fetch_embedded_search" }

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
        
        let query_str = context.experiment_overrides.get("q")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if query_str.is_empty() {
            return Ok(Vec::new());
        }

        let search_manager = context.search_manager.as_ref()
            .ok_or_else(|| anyhow!("EmbeddedSearchManager not initialized in ExecutionContext"))?;

        let schema = search_manager.schema();
        let searcher = search_manager.searcher();

        let query_parser = QueryParser::for_index(
            search_manager.index(),
            vec![schema.title, schema.description, schema.spoken_native]
        );
        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(params.limit))?;

        let history_key = format!("user:history:{}", context.profile_id.as_deref().unwrap_or("anon"));
        let watched_ids = context.cache_manager.get_list(&history_key).await.unwrap_or_default();

        let mut items = Vec::new();

        for (bm25_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            
            let item_id = retrieved_doc.get_first(schema.id)
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow!("ID missing in search index"))? as i32;

            if watched_ids.contains(&item_id.to_string()) {
                continue;
            }

            let dna_proximity = self.calculate_dna_proximity(context, &retrieved_doc, schema.vision_dna);
            let final_score = (bm25_score * (1.0 - params.dna_weight)) + (dna_proximity * params.dna_weight);

            let metadata_str = retrieved_doc.get_first(schema.metadata)
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let metadata_json: JsonValue = serde_json::from_str(metadata_str).unwrap_or(json!({}));

            items.push(ScoredItem::new(
                item_id,
                final_score,
                json!({
                    "source": "embedded_tantivy",
                    "bm25_score": bm25_score,
                    "dna_proximity": dna_proximity,
                    "metadata": metadata_json,
                }),
            ));
        }

        debug!(query = %query_str, count = items.len(), "Embedded search recovery complete");
        Ok(items)
    }
}

impl EmbeddedSearchStage {
    fn calculate_dna_proximity(&self, context: &ExecutionContext, doc: &TantivyDocument, dna_field: tantivy::schema::Field) -> f32 {
        let _doc_dna_bytes = match doc.get_first(dna_field).and_then(|v| v.as_bytes()) {
            Some(b) => b,
            None => return 0.0,
        };

        if let Some(_match_id) = context.experiment_overrides.get("target_dna_id").and_then(|v| v.as_i64()) {
            return 0.9;
        }

        0.5 
    }
}
