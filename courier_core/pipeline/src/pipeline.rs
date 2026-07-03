use crate::Middleware;

/// An assembled pipeline ready to handle requests.
///
/// Created via [`PipelineBuilder::build`](crate::PipelineBuilder::build)
/// — users cannot construct this directly. Call [`handle`](Self::handle)
/// to send a request through every middleware in sequence.
///
/// # Short-circuit
///
/// Short-circuit is achieved through [`Mixin`](crate::Mixin) wrappers.
/// A wrapper that decides **not** to call `self.inner.handle(req)`
/// immediately returns, skipping the remainder of the chain.
///
/// # Examples
///
/// ```rust,ignore
/// use pipeline::PipelineBuilder;
///
/// let pipeline = PipelineBuilder::new()
///   .middleware(core_service)
///   .build();
///
/// let resp = pipeline.handle(request).await?;
/// ```
pub struct Pipeline<M> {
  pub(crate) chain: M,
}

impl<M> Pipeline<M> {
  /// Sends `req` through every middleware in the pipeline.
  ///
  /// Returns the final response or the first error encountered. If a
  /// [`Mixin`](crate::Mixin) wrapper short-circuits, the inner chain
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
