pub mod infrastructure;
pub mod interface;

pub use infrastructure::minio::MinioStorage;
pub use infrastructure::storage_client::StorageInProcessClient;
pub use interface::router;
pub use storage_service_client::StorageClient;
