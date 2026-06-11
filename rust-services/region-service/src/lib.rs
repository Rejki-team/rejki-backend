pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod interface;

pub use interface::router;
pub use interface::RegionInProcessClient;
pub use application::service::RegionService;
pub use infrastructure::PgRegionRepository;
