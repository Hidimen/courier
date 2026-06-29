use std::error::Error;

use protocol::Protocol;

pub trait Service<P: Protocol> {
  type Error: Error + Send + Sync + 'static;

  fn handle(
    &self, ctx: P::Context,
  ) -> impl Future<Output = Result<P::Context, Self::Error>> + Send;
}
