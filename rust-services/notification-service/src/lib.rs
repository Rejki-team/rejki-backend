pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

pub use infrastructure::NotificationPublisher;
pub use interface::router;
pub use notification_service_client::NotificationClient;
