use std::{convert::Infallible, error::Error, marker::PhantomData};

use crate::{Layer, Middleware};

/// A middleware that transforms a request with a pure closure.
///
/// `Then<F, E>` applies `F` to the incoming request and returns the
/// result. The transformation never errors — `E` is a phantom type
/// parameter that the builder sets to match the preceding middleware's
/// error type, so that [`Chain`](crate::Chain) composition always
/// type-checks.
///
/// | Created via | `E` |
/// |---|---|
/// | [`PipelineBuilder::then`](crate::PipelineBuilder::then) on
/// [`Empty`](crate::Empty) | [`Infallible`] |
/// | [`PipelineBuilder::then`](crate::PipelineBuilder::then) on
/// [`NonEmpty`](crate::NonEmpty) | `M::Error` |
pub struct Then<F, E = Infallible>(pub(crate) F, pub(crate) PhantomData<E>);

impl<F> Then<F> {
  /// Creates a new [`Then`] with [`Infallible`] as the error type.
  pub fn new(f: F) -> Self {
    Then(f, PhantomData)
  }
}

impl<Req, F, B, E> Middleware<Req> for Then<F, E>
where
  Req: Send + 'static,
  F: Fn(Req) -> B + Send + Sync + 'static,
  B: Send + 'static,
  E: Error + Send + Sync + 'static,
{
  type Response = B;
  type Error = E;

  async fn handle(&self, req: Req) -> Result<Self::Response, Self::Error> {
    Ok((self.0)(req))
  }
}

// ── MapErr ─────────────────────────────────────────────────────────

/// A [`Layer`] that transforms the error type of the inner middleware.
pub struct MapErrLayer<F> {
  pub(crate) f: F,
}

impl<F> MapErrLayer<F> {
  /// Creates a new [`MapErrLayer`] with the given error-mapping closure.
  pub fn new(f: F) -> Self {
    MapErrLayer { f }
  }
}

impl<Inner, F> Layer<Inner> for MapErrLayer<F> {
  type Output = MapErr<Inner, F>;

  fn layer(self, inner: Inner) -> Self::Output {
    MapErr { inner, f: self.f }
  }
}

/// Middleware produced by [`MapErrLayer`].
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
