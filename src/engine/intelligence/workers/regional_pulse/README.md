# 🌍 Regional Pulse: Hyper-Local Intelligence Worker

The Regional Pulse worker scrapes and classifies local events to provide the engine with a real-time understanding of what is happening in the user's specific region.

---

## 🚀 Lifecycle

1. **Scrape**: Periodically fetches headlines and events for targeted regions (e.g., Nairobi, London).
2. **Classify**: Sends headlines to the `HiveMindConnector` (LLM) for categorization.
3. **Filter**: Identifies `PhysicalEvent` types (Natural Disasters, Festivals, etc.) and filters out general discourse.
4. **Vectorize**: Generates a semantic vector for high-confidence physical events.
5. **Cache**: Stores the semantic pulse in Redis: `pulse:{location} -> {theme, vector, confidence}`.

---

## 🎯 Impact

This worker enables the `BoostHyperLocalPulseStage` to apply real-time semantic boosts to content that matches current regional events, creating a "now and here" feeling for the user.

---

## ⚙️ Configuration

The scrape interval is currently set to hourly in the `DiscoverySymphony` builder.

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-regional-pulse-hyper-local-intelligence-worker)
