/// A decorator that wraps a [`Middleware`](crate::Middleware) to
/// produce a new middleware.
///
/// # Short-circuit via Mixin
///
/// The key benefit of a `Mixin` is that the wrapper **owns** the inner
/// middleware. To short-circuit the chain, simply return `Ok(resp)`
/// without calling `self.inner.handle(req)`.
///
/// Unlike [`Middleware`](crate::Middleware), `Mixin` has no request
/// parameter — the wrapping is purely a type-level transformation. The
/// request / response constraints only appear when the wrapper
/// implements [`Middleware`](crate::Middleware).
///
/// # Examples
///
/// A mixin that conditionally short-circuits:
///
/// ```rust,ignore
/// use pipeline::{Middleware, Mixin};
///
/// struct AuthMixin;
///
/// impl<M> Mixin<M> for AuthMixin {
///   type Wrapper = Auth<M>;
///
///   fn mix(self, inner: M) -> Self::Wrapper {
///     Auth { inner }
///   }
/// }
///
/// struct Auth<M> { inner: M }
///
/// impl<Req, M: Middleware<Req, Response = Req>> Middleware<Req>
///   for Auth<M>
/// {
///   type Response = Req;
///   type Error = M::Error;
///
///   async fn handle(
///     &self, req: Req,
///   ) -> Result<Self::Response, Self::Error> {
///     if !is_authenticated(&req) {
///       return Ok(req); // Short-circuit
///     }
///     self.inner.handle(req).await // Continue
///   }
/// }
/// ```
pub trait Mixin<M> {
  /// The wrapper type produced by this mixin.
  type Wrapper;

  /// Wraps `inner` and returns a new middleware.
  ///
  /// The returned wrapper may conditionally call or skip `inner`,
  /// implementing short-circuit behaviour.
  fn mix(self, inner: M) -> Self::Wrapper;
}
