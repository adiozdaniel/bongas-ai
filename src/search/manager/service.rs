//! Phase 1.3: Atomic Search Lifecycle Manager
//!
//! Manages the embedded Tantivy index, ensuring atomic commits and
//! thread-safe access to readers and writers.

use std::path::Path;
use std::sync::{Arc, Mutex};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, directory::MmapDirectory, IndexSettings};
use tracing::{info, warn};
use anyhow::{Context, Result};

use crate::search::SearchSchema;
use crate::config::types::SearchConfig;

/// High-performance manager for the embedded search index.
pub struct EmbeddedSearchManager {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    schema: SearchSchema,
    _config: SearchConfig,
}

impl EmbeddedSearchManager {
    /// Initialize the search manager, creating or opening the index.
    pub fn new(config: SearchConfig) -> Result<Self> {
        let schema_wrapper = SearchSchema::new();
        let index_path = Path::new(&config.index_path);

        // 1. Ensure index directory exists
        if !index_path.exists() {
            std::fs::create_dir_all(index_path)
                .context("Failed to create search index directory")?;
            info!(path = %config.index_path, "Created search index directory");
        }

        let directory = MmapDirectory::open(index_path)?;

        // 2. Open or Create the Index
        let index = if Index::exists(&directory)? {
            info!(path = %config.index_path, "Opening existing search index");
            Index::open(directory)?
        } else {
            info!(path = %config.index_path, "Creating new search index");
            Index::create(directory, schema_wrapper.schema.clone(), IndexSettings::default())?
        };

        // Phase 3.3: Register specialized "sheng" analyzer
        index.tokenizers().register("sheng", crate::search::sheng_analyzer());

        // 3. Initialize Reader
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        // 4. Initialize Writer (Memory budget from config)
        let writer_mem = config.writer_memory_mb * 1024 * 1024;
        let writer = index.writer(writer_mem)?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            schema: schema_wrapper,
            _config: config,
        })
    }

    /// Access the Tantivy schema fields.
    pub fn schema(&self) -> &SearchSchema {
        &self.schema
    }

    /// Access the Tantivy index object.
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Get a fresh searcher from the reader.
    pub fn searcher(&self) -> tantivy::Searcher {
        self.reader.searcher()
    }

    /// Add or update a document in the index.
    pub async fn upsert_document(&self, doc: tantivy::TantivyDocument) -> Result<()> {
        let writer = self.writer.lock().map_err(|_| anyhow::anyhow!("Search writer mutex poisoned"))?;
        writer.add_document(doc)?;
        Ok(())
    }

    /// Commit all pending changes atomically.
    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow::anyhow!("Search writer mutex poisoned"))?;
        writer.commit()?;
        // Trigger manual reload of the reader
        self.reader.reload()?;
        info!("Search index committed and reloaded successfully");
        Ok(())
    }

    /// Delete an item from the index by ID.
    pub fn delete_item(&self, id: i64) -> Result<()> {
        let writer = self.writer.lock().map_err(|_| anyhow::anyhow!("Search writer mutex poisoned"))?;
        let term = tantivy::Term::from_field_i64(self.schema.id, id);
        writer.delete_term(term);
        Ok(())
    }

    /// Trigger a full rollback of uncommitted changes.
    pub fn rollback(&self) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow::anyhow!("Search writer mutex poisoned"))?;
        writer.rollback()?;
        warn!("Search index changes rolled back");
        Ok(())
    }
}
