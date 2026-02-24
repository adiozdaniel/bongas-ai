# 🛡️ Config Type: Security

Manages sensitive security parameters, including API keys for various platforms (Web, Mobile, TV), JWT secret keys, and license validation server URLs.

---

## 🏗️ Security Perimeter

```mermaid
graph TD
    Sec[SecurityConfig] --> Keys[Platform API Keys]
    Sec --> JWT[JWT Secrets]
    Sec --> Lic[License Validation]
```

---
[⬅️ Back to Types Main](../README.md)
