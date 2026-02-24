# ⚙️ Configuration Module

> **Layered, immutable, and validated configuration backbone.**

The Configuration module implements the Netflix-grade Composite Configuration Pattern. It provides a robust system for loading, merging, and validating application settings from multiple sources with a strict precedence hierarchy.

---

## 🏗️ Architecture Overview

```mermaid
graph TD
    Toml[TOML Defaults] --> Loader[Config Loader]
    Env[Environment Vars] --> Loader
    Cloud[Spring Cloud Config] --> Loader
    
    Loader --> Merge[Layer Merging]
    Merge --> Val[Deep Validation]
    Val --> Final[Immutable AppConfig]
    
    Final --> App[Engine / API / Resilience]
```

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**📥 Loader**](./loader/README.md) | Orchestrator for the layered loading process. |
| [**📡 Sources**](./sources/README.md) | Pluggable backends (TOML, Env, Spring Cloud). |
| [**🧬 Types**](./types/README.md) | Strongly-typed configuration models. |
| [**🛡️ Validation**](./validation/README.md) | Cross-module dependency and constraint checking. |
| [**🌐 HTTP**](./http/README.md) | Web server specific settings (CORS, Compression). |

---

## 🚀 Quick Start

```rust
use config::ConfigLoader;

let config = ConfigLoader::new()
    .with_defaults()
    .with_env()
    .load()?;

println!("Server running on port: {}", config.server.port);
```

---
[🏠 Back to Project Root](../../README.md)
