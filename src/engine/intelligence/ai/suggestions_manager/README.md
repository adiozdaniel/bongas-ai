# 🤖 Engine: Suggestions Manager

The AI-driven brainstorming component. It translates natural language queries into executable strategic rules and manages the lifecycle of rule suggestions from the Analytics Sidecar.

---

## 🧠 Chatbot Translation Flow

```mermaid
graph TD
    Input[NL Query: 'Smart TV user'] --> NLP[Translation Engine]
    NLP --> Cond[Condition: context.device='tv']
    Cond --> Sugg[Rule Suggestion]
    Sugg --> Admin[Admin Approval]
    Admin --> Rule[Active Strategic Rule]
```

---
[⬅️ Back to Engine Main](../README.md)
