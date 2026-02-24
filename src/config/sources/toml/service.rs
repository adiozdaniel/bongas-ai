use crate::config::sources::{ConfigSource, ConfigResult, ConfigError};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::Value;

pub struct TomlSource {
    path: String,
}

impl TomlSource {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub fn from_path(path: String) -> Self {
        Self { path }
    }

    fn parse_toml_to_flat_map(toml_content: &str) -> ConfigResult<HashMap<String, String>> {
        let toml_value: Value = toml::from_str(toml_content)
            .map_err(|e| ConfigError::TomlParse(format!("{}", e)))?;

        let mut result = HashMap::new();
        Self::flatten_toml("", &toml_value, &mut result);
        Ok(result)
    }

    fn flatten_toml(prefix: &str, value: &Value, result: &mut HashMap<String, String>) {
        match value {
            Value::Table(table) => {
                for (key, value) in table {
                    let new_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::flatten_toml(&new_prefix, value, result);
                }
            }
            Value::Array(array) => {
                for (index, value) in array.iter().enumerate() {
                    let new_prefix = format!("{}[{}]", prefix, index);
                    Self::flatten_toml(&new_prefix, value, result);
                }
            }
            Value::String(s) => {
                result.insert(prefix.to_string(), s.clone());
            }
            Value::Integer(i) => {
                result.insert(prefix.to_string(), i.to_string());
            }
            Value::Float(f) => {
                result.insert(prefix.to_string(), f.to_string());
            }
            Value::Boolean(b) => {
                result.insert(prefix.to_string(), b.to_string());
            }
            Value::Datetime(dt) => {
                result.insert(prefix.to_string(), dt.to_string());
            }
        }
    }
}

impl ConfigSource for TomlSource {
    fn load(&self) -> ConfigResult<HashMap<String, String>> {
        let content = fs::read_to_string(&self.path)
            .map_err(ConfigError::Io)?;

        Self::parse_toml_to_flat_map(&content)
    }

    fn name(&self) -> &'static str {
        "TOML"
    }
}
