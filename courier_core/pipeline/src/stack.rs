use crate::Middleware;

/// A stack of two middlewares executed in sequence.
///
/// `C` handles the request first, producing an intermediate response
/// that feeds into `N`. Both middlewares always run — there is no
/// short-circuit between them. Use a [`Mixin`](crate::Mixin) to wrap
/// the chain if you need conditional execution.
///
/// Constructed by
/// [`PipelineBuilder::middleware`](crate::PipelineBuilder::middleware)
/// — users should not create this directly.
pub struct Stack<C, N>(pub(crate) C, pub(crate) N);

impl<Req, C, N> Middleware<Req> for Stack<C, N>
where
  Req: Send,
  C: Middleware<Req> + Sync,
  N: Middleware<C::Response> + Sync,
  N::Error: Into<C::Error>,
{
  type Response = N::Response;
  type Error = C::Error;

  async fn handle(&self, req: Req) -> Result<Self::Response, Self::Error> {
    let intermediate = self.0.handle(req).await?;
    self.1.handle(intermediate).await.map_err(Into::into)
  }
}
