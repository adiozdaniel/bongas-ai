# 📋 Search: Schema

> **The multi-modal blueprint for content discovery.**

Defines the structure of the search index, mapping raw content and latent vectors into a searchable format.

## 🗂️ Field Map

| Field | Type | Purpose |
| :--- | :--- | :--- |
| `id` | i64 | Primary Key (Postgres parity). |
| `title` | Text | Primary search attribute (Sheng-native). |
| `description` | Text | Contextual search attribute (Sheng-native). |
| `spoken_content` | Text | Spoken words extracted via Sound Listener. |
| `vision_dna` | Bytes | Stored Vision DNA for Latent Re-ranking. |
| `metadata` | JSON | Serialized UI metadata for instant delivery. |

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Search](../README.md)
