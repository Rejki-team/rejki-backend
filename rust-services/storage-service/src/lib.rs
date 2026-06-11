pub mod infrastructure;
pub mod interface;

pub use interface::router;
pub use infrastructure::storage_client::StorageInProcessClient;
pub use infrastructure::minio::MinioStorage;
