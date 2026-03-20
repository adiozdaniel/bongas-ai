//! Phase 1.2: Multi-Modal Search Schema
//!
//! Defines the structure of the embedded search index, supporting keyword matching,
//! deep-content (spoken words), and latent vision DNA for personalized re-ranking.

use tantivy::schema::*;

/// The Bongas Search Schema definition.
#[derive(Clone)]
pub struct SearchSchema {
    pub schema: Schema,
    pub id: Field,
    pub title: Field,
    pub description: Field,
    pub spoken_content: Field,
    pub vision_dna: Field,
    pub metadata: Field,
}

impl SearchSchema {
    pub fn new() -> Self {
        let mut schema_builder = Schema::builder();

        // 1. Primary Key: Item ID (i64 for Postgres parity)
        let id = schema_builder.add_i64_field("id", INDEXED | STORED);

        // 2. Metadata Search: Title and Description
        // Using TEXT with STORED so we can show snippets or results directly
        let title = schema_builder.add_text_field("title", TEXT | STORED);
        let description = schema_builder.add_text_field("description", TEXT | STORED);

        // 3. Deep Content: Spoken Words (extracted via Sound Listener)
        // We don't necessarily need to store the raw text here if it's large,
        // just index it for retrieval.
        let spoken_content = schema_builder.add_text_field("spoken_content", TEXT);

        // 4. Vision DNA: Vector Storage (for Phase 4 Relevance Fusion)
        // tantivy 0.22 doesn't have a native dense vector type yet, so we store
        // the DNA as a raw byte blob for memory-resident re-ranking.
        let vision_dna = schema_builder.add_bytes_field("vision_dna", STORED);

        // 5. Raw Metadata: JSON blob for UI flexibility
        let metadata = schema_builder.add_json_field("metadata", STORED);

        Self {
            schema: schema_builder.build(),
            id,
            title,
            description,
            spoken_content,
            vision_dna,
            metadata,
        }
    }
}

impl Default for SearchSchema {
    fn default() -> Self {
        Self::new()
    }
}
