# 🛡️ Pipeline Stages: Filter

> **Hard constraints to ensure safety, availability, and relevance.**

Filter stages remove items from the stream that do not meet specific criteria. They ensure that the final recommendations are safe, licensed, and appropriate for the user.

---

## 🧩 Sub-Modules

| Stage | Description |
| :--- | :--- |
| [**✅ Availability**](./filter_by_availability/README.md) | Removes items that are no longer active or licensed. |
| [**🔞 Maturity**](./maturity_filter/README.md) | Enforces age-rating constraints based on user profile. |
| [**🌍 Country**](./filter_by_country/README.md) | Filters content based on geographic licensing restrictions. |
| [**🗣️ Language**](./filter_by_language/README.md) | Filters content based on user's preferred or spoken languages. |
| [**🙈 Already Watched**](./filter_already_watched/README.md) | Excludes items the user has already consumed. |
| [**⚠️ Explicit Content**](./filter_explicit_content/README.md) | Safety filter for sensitive or adult-oriented material. |
| [**📏 Duration**](./filter_by_duration/README.md) | Filters items based on their length (e.g., short-form only). |
| [**🏷️ Genre**](./filter_by_genre/README.md) | Restricts items to a specific set of allowed genres. |
| [**🌟 Quality**](./filter_by_quality/README.md) | Filters items based on resolution or production quality. |
| [**📊 Rating**](./filter_by_rating/README.md) | Excludes items below a certain user or critic rating threshold. |
| [**📅 Release Year**](./filter_by_release_year/README.md) | Filters items based on when they were published. |
| [**💳 Subscription**](./filter_by_subscription_tier/README.md) | Filters items based on the user's payment tier. |
| [**👶 Age Rating**](./filter_by_age_rating/README.md) | Enforces specific rating codes (G, PG, R). |

---
[⬅️ Back to Stages Main](../README.md)
