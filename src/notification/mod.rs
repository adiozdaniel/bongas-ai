//! Notification System: Orchestrates engagement across Email and In-App channels.

pub mod models;
pub mod repository;
pub mod dispatcher;

pub use dispatcher::service::NotificationDispatcher;
pub use repository::service::NotificationRepository;
