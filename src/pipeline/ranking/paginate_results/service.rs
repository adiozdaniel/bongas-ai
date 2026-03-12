use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    page: usize,
    /// Items per page
    #[serde(default = "default_page_size")]
    page_size: usize,
    /// Optional cursor for cursor-based pagination
    #[serde(default)]
    cursor: Option<String>,
    /// Include pagination metadata in results
    #[serde(default = "default_true")]
    include_metadata: bool,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

fn default_true() -> bool {
    true
}

pub struct PaginateResultsStage;

#[async_trait]
impl PipelineStage for PaginateResultsStage {
    fn name(&self) -> &str {
        "paginate_results"
    }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let total_items = input.len();
        let total_pages = total_items.div_ceil(params.page_size);

        // Handle cursor-based pagination
        let start_index = if let Some(ref cursor) = params.cursor {
            // Cursor format: "item_id:score" - find position after this item
            let parts: Vec<&str> = cursor.split(':').collect();
            if parts.len() == 2 {
                if let Ok(cursor_id) = parts[0].parse::<i32>() {
                    input.iter()
                        .position(|item| item.item_id == cursor_id)
                        .map(|pos| pos + 1)
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            // Offset-based pagination
            let page = params.page.max(1);
            (page - 1) * params.page_size
        };

        let end_index = (start_index + params.page_size).min(total_items);

        // Extract page of results
        let page_items: Vec<ScoredItem> = input
            .into_iter()
            .skip(start_index)
            .take(params.page_size)
            .enumerate()
            .map(|(idx, mut item)| {
                if params.include_metadata {
                    // Add pagination metadata to each item
                    let mut metadata = item.metadata.as_object()
                        .cloned()
                        .unwrap_or_default();

                    metadata.insert("_pagination".to_string(), json!({
                        "position": start_index + idx + 1,
                        "total_items": total_items,
                        "total_pages": total_pages,
                        "current_page": params.page,
                        "page_size": params.page_size,
                        "has_next": end_index < total_items,
                        "has_prev": start_index > 0,
                        "next_cursor": if end_index < total_items {
                            Some(format!("{}:{}", item.item_id, item.score))
                        } else {
                            None
                        },
                    }));

                    item.metadata = json!(metadata);
                }
                item
            })
            .collect();

        Ok(page_items)
    }
}
