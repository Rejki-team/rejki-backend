pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

pub use application::service::ChatService;
pub use auth_service_client::AuthClient;
pub use chat_service_client::{ChatClient, ChatClientError};
pub use infrastructure::{ChatInProcessClient, PgChatRepository};
pub use interface::router;
