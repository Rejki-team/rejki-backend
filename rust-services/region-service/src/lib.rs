pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

pub use application::service::RegionService;
pub use infrastructure::PgRegionRepository;
pub use interface::router;
pub use interface::RegionInProcessClient;
pub use region_service_client::RegionClient;
