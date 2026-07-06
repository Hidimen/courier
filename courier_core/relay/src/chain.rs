use crate::Middleware;

/// Two middlewares composed in unconditional sequence.
///
/// `Left` handles the request first, producing an intermediate response
/// that feeds into `Right`. Both middlewares always run — there is no
/// short-circuit between them. Use a [`Layer`](crate::Layer) to wrap the
/// chain if you need conditional execution.
///
/// Constructed by
/// [`PipelineBuilder::middleware`](crate::PipelineBuilder::middleware) —
/// users should not create this directly.
pub struct Chain<Left, Right>(pub(crate) Left, pub(crate) Right);

impl<Req, Left, Right> Middleware<Req> for Chain<Left, Right>
where
  Req: Send,
  Left: Middleware<Req> + Sync,
  Right: Middleware<Left::Response> + Sync,
  Right::Error: Into<Left::Error>,
{
  type Response = Right::Response;
  type Error = Left::Error;

  async fn handle(&self, req: Req) -> Result<Self::Response, Self::Error> {
    let intermediate = self.0.handle(req).await?;
    self.1.handle(intermediate).await.map_err(Into::into)
  }
}
