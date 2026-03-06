  //! Telemetry exporters for various output targets.
  //!
  //! Provides exporters for stdout, stderr, file, and future OTLP support.

  use std::fs::{File, OpenOptions};
  use std::io::{self, Write};
  use std::path::Path;
  use std::sync::{Arc, Mutex, PoisonError};

  use opentelemetry::{trace::TracerProvider as _, KeyValue};
  use opentelemetry_otlp::WithExportConfig;
  use opentelemetry_sdk::{
      runtime,
      trace::{self, Sampler, TracerProvider},
      Resource,
  };

  // ─── Writer Trait ───────────────────────────────────────────────────────────

  /// Trait for writers that can be used with tracing.
  pub trait TelemetryWriter: Write + Send + Sync + 'static {}

  impl<T: Write + Send + Sync + 'static> TelemetryWriter for T {}

  // ─── File Writer ────────────────────────────────────────────────────────────

  /// Thread-safe file writer with automatic rotation support.
  pub struct FileWriter {
      file: Arc<Mutex<File>>,
      path: std::path::PathBuf,
  }

  impl FileWriter {
      /// Create a new file writer.
      pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
          let path = path.as_ref().to_path_buf();
          let file = OpenOptions::new()
              .create(true)
              .append(true)
              .open(&path)?;

          Ok(Self {
              file: Arc::new(Mutex::new(file)),
              path,
          })
      }

      /// Get the file path.
      pub fn path(&self) -> &Path {
          &self.path
      }

      /// Rotate the log file (rename current, create new).
      pub fn rotate(&self) -> io::Result<()> {
          let timestamp = std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)
              .map(|d| d.as_secs())
              .unwrap_or(0);

          let rotated_path = self.path.with_extension(format!("log.{}", timestamp));

          // Rename current file
          std::fs::rename(&self.path, rotated_path)?;

          // Create new file
          let new_file = OpenOptions::new()
              .create(true)
              .append(true)
              .open(&self.path)?;

          // Replace the file handle
          let mut guard = self.file.lock().unwrap_or_else(PoisonError::into_inner);
          *guard = new_file;

          Ok(())
      }
  }

  impl Write for FileWriter {
      fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
          let mut guard = self.file.lock().unwrap_or_else(PoisonError::into_inner);
          guard.write(buf)
      }

      fn flush(&mut self) -> io::Result<()> {
          let mut guard = self.file.lock().unwrap_or_else(PoisonError::into_inner);
          guard.flush()
      }
  }

  impl Clone for FileWriter {
      fn clone(&self) -> Self {
          Self {
              file: Arc::clone(&self.file),
              path: self.path.clone(),
          }
      }
  }

  // ─── Multi Writer ───────────────────────────────────────────────────────────

  /// Writer that fans out to multiple writers.
  pub struct MultiWriter {
      writers: Vec<Box<dyn Write + Send + Sync>>,
  }

  impl MultiWriter {
      /// Create a new multi-writer.
      pub fn new() -> Self {
          Self {
              writers: Vec::new(),
          }
      }

      /// Add a writer to the multi-writer.
      pub fn with_writer<W: Write + Send + Sync + 'static>(mut self, writer: W) -> Self {
          self.writers.push(Box::new(writer));
          self
      }
  }

  impl Default for MultiWriter {
      fn default() -> Self {
          Self::new()
      }
  }

  impl Write for MultiWriter {
      fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
          for writer in &mut self.writers {
              writer.write_all(buf)?;
          }
          Ok(buf.len())
      }

      fn flush(&mut self) -> io::Result<()> {
          for writer in &mut self.writers {
              writer.flush()?;
          }
          Ok(())
      }
  }

  // ─── Null Writer ────────────────────────────────────────────────────────────

  /// Writer that discards all output.
  pub struct NullWriter;

  impl Write for NullWriter {
      fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
          Ok(buf.len())
      }

      fn flush(&mut self) -> io::Result<()> {
          Ok(())
      }
  }

  // ─── Buffered Writer ────────────────────────────────────────────────────────

  /// Buffered writer with configurable flush interval.
  pub struct BufferedWriter<W: Write> {
      inner: W,
      buffer: Vec<u8>,
      capacity: usize,
  }

  impl<W: Write> BufferedWriter<W> {
      /// Create a new buffered writer with the given capacity.
      pub fn new(writer: W, capacity: usize) -> Self {
          Self {
              inner: writer,
              buffer: Vec::with_capacity(capacity),
              capacity,
          }
      }

      /// Flush if buffer is at capacity.
      fn maybe_flush(&mut self) -> io::Result<()> {
          if self.buffer.len() >= self.capacity {
              self.flush()?;
          }
          Ok(())
      }
  }

  impl<W: Write> Write for BufferedWriter<W> {
      fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
          self.buffer.extend_from_slice(buf);
          self.maybe_flush()?;
          Ok(buf.len())
      }

      fn flush(&mut self) -> io::Result<()> {
          if !self.buffer.is_empty() {
              self.inner.write_all(&self.buffer)?;
              self.inner.flush()?;
              self.buffer.clear();
          }
          Ok(())
      }
  }

  impl<W: Write> Drop for BufferedWriter<W> {
      fn drop(&mut self) {
          let _ = self.flush();
      }
  }

  // ─── OTLP Exporter (Live) ───────────────────────────────────────────────────

  /// Supported OTLP transport protocols.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum OtlpProtocol {
      /// OTLP over gRPC (high performance).
      Grpc,
      /// OTLP over HTTP/JSON (better firewall compatibility).
      #[default]
      Http,
  }

  /// Configuration for the OTLP exporter.
  #[derive(Debug, Clone)]
  pub struct OtlpExporterConfig {
      /// OTLP endpoint URL.
      pub endpoint: String,
      /// The transport protocol to use.
      pub protocol: OtlpProtocol,
      /// Headers to include in requests (e.g., API keys).
      pub headers: Vec<(String, String)>,
      /// Request timeout in milliseconds.
      pub timeout_ms: u64,
      /// Batch size for exporting spans.
      pub batch_size: usize,
      /// Maximum queue size for the span processor.
      pub max_queue_size: usize,
  }

  impl Default for OtlpExporterConfig {
      fn default() -> Self {
          Self {
              endpoint: String::from("http://localhost:4318/v1/traces"),
              protocol: OtlpProtocol::Http,
              headers: Vec::new(),
              timeout_ms: 10000,
              batch_size: 512,
              max_queue_size: 2048,
          }
      }
  }
