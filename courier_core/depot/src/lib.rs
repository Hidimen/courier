mod builder;
pub mod error;
pub mod server;

pub use builder::DepotBuilder;
pub use error::DepotBuildError;
pub use server::Depot;
