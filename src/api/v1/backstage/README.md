# 🔐 THE BACKSTAGE: Administrative Control

The Backstage is the command center for the Bongas-AI engine. It provides high-integrity endpoints for managing the discovery graph, ML strategies, and system logic.

[🏠 Hub](../../../../docs/HUB.md) | [🏗️ Architecture](../../../../docs/architecture/SYMPHONY.md) | [🎨 Smart Pages](../../../../docs/architecture/PAGES.md)

---

## 🏗️ Functional Domains

### 🎨 Orchestration (`orchestration.rs`)

Management of the **Navigation Mesh** and **SDUI Layouts**.

- **Layout CRUD**: Create and update structured compositions with `row_type` and `row_style`.
- **Targeting Rules**: Configure `is_landing` flags and contextual overrides per device/maturity.

### 🧠 Strategy (`strategy.rs`)

Governance of **Scenarios** and **ML Pipelines**.

- **Pipeline Recipes**: Define the selected stages, rankers, and diversity filters for each scenario.
- **Safety Ceilings**: Enforce KFCB-compliant maturity ratings at the scenario level.

### 🤖 Intelligence (`intelligence.rs`)

Control over the **Intelligent Brain** and feature store.

- **ML Suggestions**: Review and approve AI-generated rule optimizations.
- **Chatbot Interface**: Query and modify engine state using natural language.

---

## ⚡ Atomic Governance

The Backstage implements the **Atomic Swap** pattern. Administrators can perform a bulk reload of all scenarios and layouts without downtime, ensuring the engine transitions from "Old World" to "New World" in a single CPU cycle.

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-the-backstage-administrative-control)
