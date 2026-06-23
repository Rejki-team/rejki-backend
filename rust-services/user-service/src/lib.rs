pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

pub use application::service::UserService;
pub use infrastructure::PgUserRepository;
pub use interface::router;
pub use interface::UserInProcessClient;
pub use user_service_client::UserClient;
