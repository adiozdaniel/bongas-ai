  //! No-op cache for testing.

  use crate::cache::{CacheStrategy, CacheTier};
  use async_trait::async_trait;
  use serde::{Deserialize, Serialize};
  use std::time::Duration;
  use anyhow::Result;

  /// No-op cache that does nothing (for testing).
  pub struct NoOpCache;

  impl NoOpCache {
      pub fn new() -> Self {
          Self
      }
  }

  impl Default for NoOpCache {
      fn default() -> Self {
          Self::new()
      }
  }

  #[async_trait]
  impl CacheStrategy for NoOpCache {
      async fn get<T>(&self, _key: &str) -> Result<Option<T>>
      where
          T: for<'de> Deserialize<'de> + Send,
      {
          Ok(None)
      }

      async fn set<T>(&self, _key: &str, _value: &T, _ttl: Duration) -> Result<()>
      where
          T: Serialize + Send + Sync,
      {
          Ok(())
      }

      async fn push_to_list(&self, _key: &str, _value: String, _max_len: usize) -> Result<()> {
          Ok(())
      }

      async fn get_list(&self, _key: &str) -> Result<Vec<String>> {
          Ok(Vec::new())
      }

      async fn set_raw(&self, _key: &str, _value: String, _ttl: Duration) -> Result<()> {
          Ok(())
      }

      async fn get_raw(&self, _key: &str) -> Result<Option<String>> {
          Ok(None)
      }

      async fn delete(&self, _key: &str) -> Result<()> {
          Ok(())
      }

      async fn delete_pattern(&self, _pattern: &str) -> Result<()> {
          Ok(())
      }

      async fn exists(&self, _key: &str) -> Result<bool> {
          Ok(false)
      }

      async fn clear(&self) -> Result<()> {
          Ok(())
      }

      fn name(&self) -> &'static str {
          "noop"
      }

      fn tier(&self) -> CacheTier {
          CacheTier::L3
      }
  }

