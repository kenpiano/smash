pub mod config;
pub mod error;
pub mod load;
pub mod merge;
pub mod validate;

pub use config::{Config, LspConfig, LspServerEntry};
pub use error::ConfigError;
pub use load::{load_config, load_from_str};
