use crate::Middleware;

/// An assembled pipeline ready to handle requests.
///
/// Created via
/// [`Builder::build`](crate::Builder::build) — users
/// cannot construct this directly. Call [`handle`](Self::handle) to send
/// a request through every middleware in sequence.
///
/// # Short-circuit
///
/// Short-circuit is achieved through [`Layer`](crate::Layer) wrappers.
/// A wrapper that decides **not** to call `self.inner.handle(req)`
/// immediately returns, skipping the remainder of the chain.
///
/// # Examples
///
/// ```rust
/// use relay::{Builder, Middleware};
///
/// struct CoreService;
///
/// impl<Req: Send> Middleware<Req> for CoreService {
///   type Response = Req;
///   type Error = std::convert::Infallible;
///
///   async fn handle(&self, req: Req) -> Result<Self::Response, Self::Error> {
///     Ok(req)
///   }
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// let pipeline = Builder::new()
///   .middleware(CoreService)
///   .build();
///
/// let resp = pipeline.handle("hello").await.unwrap();
/// assert_eq!(resp, "hello");
/// # }
/// ```
pub struct Pipeline<M> {
  pub(crate) chain: M,
}

impl<M> Pipeline<M> {
  /// Sends `req` through every middleware in the pipeline.
  ///
  /// Returns the final response or the first error encountered. If a
  /// [`Layer`](crate::Layer) wrapper short-circuits, the inner chain
  /// is skipped.
  ///
  /// The request type `Req` is inferred from the argument. Trait bounds
  /// were already validated by the builder.
  #[must_use = "pipeline handles are async and must be awaited"]
  pub async fn handle<Req>(&self, req: Req) -> Result<M::Response, M::Error>
  where
    M: Middleware<Req> + Sync,
  {
    self.chain.handle(req).await
  }
}
