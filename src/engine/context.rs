use crate::pipeline::context::ExecutionContext;

/// Engine-level request context wrapping pipeline context
/// with additional metadata for tracing and analytics.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub execution_context: ExecutionContext,
    pub request_id: String,
    pub scenario_slug: String,
    pub user_id: Option<i32>,
    pub trace_id: Option<String>,
}

impl RequestContext {
    pub fn new(
        execution_context: ExecutionContext,
        request_id: String,
        scenario_slug: String,
        user_id: Option<i32>,
    ) -> Self {
        Self {
            execution_context,
            request_id,
            scenario_slug,
            user_id,
            trace_id: None,
        }
    }
}
