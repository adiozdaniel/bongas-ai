# 📦 Analytics Uploader: High-Throughput Delivery

The Analytics Uploader sub-module handles the resilient delivery of telemetry and harvested training data to central servers.

---

## 🛠 Components

### 1. StatsUploader
Handles the real-time upload of `ClientStatsPayload` (CPU, Memory, Latency, Error rates) using the **Netflix Hystrix** circuit breaker pattern.

### 2. ParquetExporter
A high-performance utility that converts raw ClickHouse/PostgreSQL interaction data into compressed **Apache Parquet** files.
- **Compression**: Uses Zstd for maximum storage efficiency.
- **Format**: Implements the Apache Arrow schema for seamless integration with Python (Pandas/PyArrow).

---

## 🚀 Execution Flow

1. **Harvest**: `TrainingOrchestrator` fetches interaction sequences from ClickHouse.
2. **Serialize**: `ParquetExporter` maps Rust structs into Arrow record batches.
3. **Persist**: Writes the results to `.parquet` files for "Backstage Training."

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Analytics](../README.md)
