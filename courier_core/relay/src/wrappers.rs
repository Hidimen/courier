use std::error::Error;

use crate::Middleware;

/// A middleware wrapper that transforms the response of an inner
/// middleware using a closure.
///
/// Created via
/// [`PipelineBuilder::then`](crate::PipelineBuilder::then). Converts
/// `M::Response` into `B`, enabling the next middleware in the chain to
/// accept a different request type.
///
/// # Examples
///
/// ```rust,ignore
/// use relay::PipelineBuilder;
///
/// let pipeline = PipelineBuilder::new()
///   .middleware(parse_int)          // Response = i32
///   .then(|n: i32| n.to_string())   // Response = String
///   .chain(process_string)          // Request = String
///   .build();
/// ```
pub struct Then<M, F> {
  pub(crate) inner: M,
  pub(crate) f: F,
}

impl<Req, M, F, B> Middleware<Req> for Then<M, F>
where
  Req: Send + 'static,
  M: Middleware<Req>,
  F: Fn(M::Response) -> B + Send + Sync + 'static,
  B: Send + 'static,
{
  type Response = B;
  type Error = M::Error;

  async fn handle(&self, req: Req) -> Result<Self::Response, Self::Error> {
    let resp = self.inner.handle(req).await?;
    Ok((self.f)(resp))
  }
}

/// A middleware wrapper that transforms the error of an inner middleware
/// using a closure.
///
/// Created via
/// [`PipelineBuilder::map_err`](crate::PipelineBuilder::map_err).
/// Converts `M::Error` into `E2`, enabling the error type to be unified
/// across middleware boundaries.
///
/// # Examples
///
/// ```rust,ignore
/// use relay::PipelineBuilder;
///
/// let pipeline = PipelineBuilder::new()
///   .middleware(my_service)           // Error = MyError
///   .map_err(|e| Box::new(e))         // Error = Box<dyn Error>
///   .build();
/// ```
pub struct MapErr<M, F> {
  pub(crate) inner: M,
  pub(crate) f: F,
}

impl<Req, M, F, E2> Middleware<Req> for MapErr<M, F>
where
  Req: Send + 'static,
  M: Middleware<Req>,
  F: Fn(M::Error) -> E2 + Send + Sync + 'static,
  E2: Error + Send + Sync + 'static,
{
  type Response = M::Response;
  type Error = E2;

  async fn handle(&self, req: Req) -> Result<Self::Response, Self::Error> {
    self.inner.handle(req).await.map_err(|e| (self.f)(e))
  }
}
