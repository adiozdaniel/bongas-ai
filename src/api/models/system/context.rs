use serde::Deserialize;

/// Shared context parameters used across various recommendation and engine endpoints.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ContextParams {
    /// The ID of the user (use 0 or None for anonymous).
    pub user_id: Option<i32>,
    /// Unique identifier for the user profile.
    pub profile_id: Option<String>,
    /// Content maturity rating filter.
    pub maturity_rating: Option<String>,
    /// Device type for platform-specific optimizations.
    pub device_type: Option<String>,
    /// Unique identifier for the visitor (unlogged or logged).
    pub visitor_id: Option<String>,
    /// Hash of the device metadata (IP + User-Agent).
    pub device_hash: Option<String>,
    /// IP address of the requesting client.
    pub ip_address: Option<String>,
}

impl ContextParams {
    /// Merge extracted identity information into the context parameters.
    pub fn merge_identity(&mut self, identity: &crate::api::middleware::identity::IdentityContext) {
        self.visitor_id = Some(identity.visitor_id.clone());
        self.device_hash = Some(identity.device_hash.clone());
        self.ip_address = Some(identity.ip_address.clone());
        
        // Prioritize detected device_type if not explicitly provided in query params
        if self.device_type.is_none() {
            self.device_type = Some(identity.device_type.clone());
        }
    }
}
