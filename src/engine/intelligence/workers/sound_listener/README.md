# 👂 Worker: Sound Listener

> **Deep-content extraction via audio intelligence.**

The Sound Listener is a background worker that monitors the content catalog for unindexed videos. It performs audio-to-text inference to extract spoken words, enabling users to search for videos based on the actual words spoken inside them.

## 🔄 Workflow

1. **Differential Census:** Polls for videos that have not yet been audio-indexed.
2. **ML Inference:** Executes audio-to-text reflexes (e.g., Whisper-Tiny) to extract transcripts.
3. **Forensic Sync:** Pushes raw transcripts to ClickHouse for analytical auditing.
4. **Index Sync:** Updates the Embedded Search index with spoken keywords.

---

[🏠 Hub](../../../../../docs/HUB.md) | [⬅️ Back to Workers](../README.md)
