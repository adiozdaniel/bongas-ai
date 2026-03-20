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

        // Phase 3.3: Use specialized "sheng" analyzer for text fields
        let text_options = TextOptions::default()
            .set_indexing_options(TextFieldIndexing::default()
                .set_tokenizer("sheng")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions))
            .set_stored();

        // 1. Primary Key: Item ID
        let id = schema_builder.add_i64_field("id", INDEXED | STORED);

        // 2. Metadata Search: Title and Description (Sheng-Native)
        let title = schema_builder.add_text_field("title", text_options.clone());
        let description = schema_builder.add_text_field("description", text_options.clone());

        // 3. Deep Content: Spoken Words (Sheng-Native)
        let spoken_content = schema_builder.add_text_field("spoken_content", text_options);

        // 4. Vision DNA: Vector Storage (Stored as bytes)
        let vision_dna = schema_builder.add_bytes_field("vision_dna", STORED);

        // 5. Raw Metadata: JSON blob (Stored as a string in 0.22)
        let metadata = schema_builder.add_text_field("metadata", STORED);

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
