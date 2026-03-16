# ⚙️ Notification Configuration

Configuration types for the Sovereign Outbound Engagement Hub.

---

## 🏗️ Models

### `NotificationConfig`

Controls the global behavior of the notification pillar.

| Field | Type | Description |
| :--- | :--- | :--- |
| `enabled` | `bool` | Master switch for the notification system. |
| `adaptor` | `enum` | The delivery channel (`kafka`, `polling`, `resend`). |
| `resend` | `ResendConfig` | Specific settings for the Resend provider. |

### `ResendConfig`

Required settings for production email delivery.

| Field | Type | Description |
| :--- | :--- | :--- |
| `api_key` | `string` | The API Token from the Resend Dashboard. |
| `from_email` | `string` | The authorized sender address. |
| `from_name` | `string` | The display name for outgoing mail. |

---

## 📄 Example Configuration

```toml
[notifications]
enabled = true
adaptor = "resend"

[notifications.resend]
api_key = "re_123456789"
from_email = "recommendations@yourdomain.com"
from_name = "Bongas Discovery"
```

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Config Main](../README.md)
