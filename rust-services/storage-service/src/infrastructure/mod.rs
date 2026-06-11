pub mod minio;
pub mod storage_client;

pub use minio::MinioStorage;
pub use storage_client::StorageInProcessClient;
