// API error handling is now fully provided by AppError (src/error/types.rs).
//
// AppError implements IntoResponse with ErrorClassifier-driven HTTP status mapping.
// All handlers return Result<Json<StandardResponse<T>>, AppError>.
//
// This file is intentionally minimal — kept only so `pub mod error;` in api/mod.rs
// continues to resolve. If desired, re-export AppError here for convenience.

pub use crate::error::AppError;
