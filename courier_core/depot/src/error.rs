use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
  #[error("Unknown kind code: {0}")]
  UnknownKindCode(u16),
}
