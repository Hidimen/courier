use depot::control::standard;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResponseError {
  #[error("Unknown kind code: {0}")]
  UnknownKindCode(u16),
  #[error("Failed to parse pod list: {0}")]
  PodListParseError(#[from] standard::error::PodParseError),
}
