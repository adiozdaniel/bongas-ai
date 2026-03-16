use std::sync::Arc;
use std::fs::File;
use std::path::Path;
use anyhow::{Result, Context};
use parquet::arrow::arrow_writer::ArrowWriter;
use arrow_array::{Int32Array, StringArray, UInt64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};

use crate::ml::training::orchestration::service::HarvestedInteraction;

/// 📦 Parquet Exporter: High-performance telemetry serialization.
pub struct ParquetExporter;

impl ParquetExporter {
    /// Converts a batch of HarvestedInteractions to a Parquet file.
    pub fn export_to_parquet(
        interactions: &[HarvestedInteraction],
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        if interactions.is_empty() {
            return Ok(());
        }

        // 1. Define Arrow Schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("user_id", DataType::Int32, false),
            Field::new("item_id", DataType::Int32, false),
            Field::new("interaction_type", DataType::Utf8, false),
            Field::new("device_type", DataType::Utf8, false),
            Field::new("profile_id", DataType::Utf8, false),
            Field::new("maturity_rating", DataType::Utf8, false),
            Field::new("genre", DataType::Utf8, false),
            Field::new("watch_duration_seconds", DataType::Int32, false),
            Field::new("created_at", DataType::UInt64, false),
        ]));

        // 2. Prepare Arrow Arrays
        let user_ids = Int32Array::from(interactions.iter().map(|i| i.user_id).collect::<Vec<_>>());
        let item_ids = Int32Array::from(interactions.iter().map(|i| i.item_id).collect::<Vec<_>>());
        let interaction_types = StringArray::from(interactions.iter().map(|i| i.interaction_type.as_str()).collect::<Vec<_>>());
        let device_types = StringArray::from(interactions.iter().map(|i| i.device_type.as_str()).collect::<Vec<_>>());
        let profile_ids = StringArray::from(interactions.iter().map(|i| i.profile_id.as_str()).collect::<Vec<_>>());
        let maturity_ratings = StringArray::from(interactions.iter().map(|i| i.maturity_rating.as_str()).collect::<Vec<_>>());
        let genres = StringArray::from(interactions.iter().map(|i| i.genre.as_str()).collect::<Vec<_>>());
        let durations = Int32Array::from(interactions.iter().map(|i| i.watch_duration_seconds).collect::<Vec<_>>());
        let created_ats = UInt64Array::from(interactions.iter().map(|i| i.created_at).collect::<Vec<_>>());

        // 3. Create RecordBatch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(user_ids),
                Arc::new(item_ids),
                Arc::new(interaction_types),
                Arc::new(device_types),
                Arc::new(profile_ids),
                Arc::new(maturity_ratings),
                Arc::new(genres),
                Arc::new(durations),
                Arc::new(created_ats),
            ],
        )?;

        // 4. Write to Parquet File
        let file = File::create(output_path).context("Failed to create Parquet file")?;
        let props = parquet::file::properties::WriterProperties::builder()
            .set_compression(parquet::basic::Compression::ZSTD(parquet::basic::ZstdLevel::default()))
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        Ok(())
    }
}
