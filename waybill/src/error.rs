use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResponseError {
  #[error("Unknown kind code: {0}")]
  UnknownKindCode(u16),
}
