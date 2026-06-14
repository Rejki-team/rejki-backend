pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

pub use interface::router;
pub use interface::ReportInProcessClient;
pub use report_service_client::ReportClient;
