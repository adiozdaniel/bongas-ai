# 📟 Config Source: Environment

Loads configuration from system environment variables. This source is typically used for production secrets and deployment-specific overrides.

---

## 🛠️ Key Mapping

| Env Variable | Config Path |
| :--- | :--- |
| `SERVER__PORT` | `server.port` |
| `DATABASE__URL` | `database.url` |
| `SECURITY__JWT_SECRET_KEY` | `security.jwt_secret_key` |

---

## 🔑 Key Features

- **Double Underscore Mapping**: Automatically converts `X__Y` to `x.y`.
- **Case Insensitivity**: Normalizes environment keys to lowercase for internal consistency.
- **Prefix Support**: Optionally filter variables by a specific prefix (e.g., `BONGAS_`).

---
[⬅️ Back to Sources Main](../README.md)
