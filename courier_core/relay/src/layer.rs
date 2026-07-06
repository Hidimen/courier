/// A decorator that wraps an inner middleware to produce a new
/// middleware.
///
/// This is a type-level factory: given an inner middleware of type
/// `Inner`, the layer produces an output middleware that owns it. The
/// output may short-circuit by not calling the inner, or do pre/post
/// processing around the inner call.
///
/// # Short-circuit
///
/// The returned wrapper owns the inner middleware. To short-circuit,
/// simply return a response without calling `self.inner.handle(req)`.
///
/// # Layer vs Middleware
///
/// A [`Layer`] is NOT a [`Middleware`](crate::Middleware) — it does not
/// handle requests itself. Instead, it **produces** a middleware by
/// wrapping another. This separation keeps the [`Layer`] reusable: the
/// same layer can wrap any compatible inner middleware.
///
/// # Examples
///
/// ## Timing layer — pre/post processing
///
/// A layer that measures how long the inner middleware takes. It always
/// calls the inner — no short-circuit — but adds timing around it:
///
/// ```rust,no_run
/// use std::time::Instant;
///
/// use relay::{Layer, Middleware};
///
/// struct TimingLayer;
///
/// impl<Inner> Layer<Inner> for TimingLayer {
///   type Output = Timing<Inner>;
///
///   fn layer(self, inner: Inner) -> Self::Output {
///     Timing { inner }
///   }
/// }
///
/// struct Timing<Inner> {
///   inner: Inner,
/// }
///
/// impl<Req, Inner> Middleware<Req> for Timing<Inner>
/// where
///   Req: Send,
///   Inner: Middleware<Req>,
/// {
///   type Response = Inner::Response;
///   type Error = Inner::Error;
///
///   async fn handle(
///     &self,
///     req: Req,
///   ) -> Result<Self::Response, Self::Error> {
///     let start = Instant::now();
///     let result = self.inner.handle(req).await;
///     let elapsed = start.elapsed();
///     println!("request took {elapsed:?}");
///     result
///   }
/// }
/// ```
///
/// ## Gate layer — short-circuit
///
/// A layer that conditionally blocks requests from reaching the inner
/// chain:
///
/// ```rust,no_run
/// use relay::{Layer, Middleware};
///
/// struct GateLayer {
///   allow: bool,
/// }
///
/// impl<Inner> Layer<Inner> for GateLayer {
///   type Output = Gate<Inner>;
///
///   fn layer(self, inner: Inner) -> Self::Output {
///     Gate { inner, allow: self.allow }
///   }
/// }
///
/// struct Gate<Inner> {
///   inner: Inner,
///   allow: bool,
/// }
///
/// impl<Req, Inner> Middleware<Req> for Gate<Inner>
/// where
///   Req: Send,
///   Inner: Middleware<Req>,
/// {
///   type Response = Inner::Response;
///   type Error = Inner::Error;
///
///   async fn handle(
///     &self,
///     req: Req,
///   ) -> Result<Self::Response, Self::Error> {
///     if !self.allow {
///       // Short-circuit — inner never runs
///       return Ok(create_blocked_response());
///     }
///     self.inner.handle(req).await
///   }
/// }
///
/// fn create_blocked_response<Resp>() -> Resp {
///   todo!()
/// }
/// ```
///
/// ## Composing layers and middleware
///
/// Layers wrap in **LIFO** order (last added = outermost = runs first).
/// Middleware appends in **FIFO** order:
///
/// ```rust,no_run
/// use relay::PipelineBuilder;
///
/// # struct Handler;
/// # #[derive(Debug)] struct AppError;
/// # impl std::fmt::Display for AppError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "err") } }
/// # impl std::error::Error for AppError {}
/// # impl<Req: Send> relay::Middleware<Req> for Handler {
/// #   type Response = Req; type Error = AppError;
/// #   async fn handle(&self, r: Req) -> Result<Req, Self::Error> { Ok(r) }
/// # }
/// # struct AuthLayer;
/// # impl<Inner> relay::Layer<Inner> for AuthLayer {
/// #   type Output = Auth<Inner>;
/// #   fn layer(self, inner: Inner) -> Self::Output { Auth { inner } }
/// # }
/// # struct Auth<Inner> { inner: Inner }
/// # impl<Req: Send, Inner: relay::Middleware<Req>> relay::Middleware<Req> for Auth<Inner> {
/// #   type Response = Inner::Response; type Error = Inner::Error;
/// #   async fn handle(&self, r: Req) -> Result<Self::Response, Self::Error> { self.inner.handle(r).await }
/// # }
/// # struct TimingLayer;
/// # impl<Inner> relay::Layer<Inner> for TimingLayer {
/// #   type Output = Timing<Inner>;
/// #   fn layer(self, inner: Inner) -> Self::Output { Timing { inner } }
/// # }
/// # struct Timing<Inner> { inner: Inner }
/// # impl<Req: Send, Inner: relay::Middleware<Req>> relay::Middleware<Req> for Timing<Inner> {
/// #   type Response = Inner::Response; type Error = Inner::Error;
/// #   async fn handle(&self, r: Req) -> Result<Self::Response, Self::Error> { self.inner.handle(r).await }
/// # }
///
/// let pipeline = PipelineBuilder::new()
///   .middleware(Handler)                        // innermost
///   .layer(AuthLayer)                           // wraps Handler
///   .layer(TimingLayer)                         // outermost
///   .build();
///
/// // Execution: Timing → Auth → Handler
/// //
/// //   ┌──────────────────────────────────┐
/// //   │ Timing                           │
/// //   │  ┌────────────────────────────┐  │
/// //   │  │ Auth (can short-circuit)   │  │
/// //   │  │  ┌──────────────────────┐  │  │
/// //   │  │  │ Handler              │  │  │
/// //   │  │  └──────────────────────┘  │  │
/// //   │  └────────────────────────────┘  │
/// //   └──────────────────────────────────┘
/// //
/// // 1. Timing  starts the clock
/// // 2. Auth    checks credentials; may skip Handler
/// // 3. Handler processes the request (if Auth allowed it)
/// // 4. Timing  prints elapsed time
/// ```
///
/// **Note**: the wrapper struct (e.g. `Timing<Inner>`, `Auth<Inner>`)
/// is what actually implements [`Middleware`](crate::Middleware). The
/// layer (e.g. `TimingLayer`, `AuthLayer`) is just the factory — it
/// implements [`Layer<Inner>`] and produces the wrapper via
/// [`layer`](Layer::layer).
pub trait Layer<Inner> {
  /// The middleware type produced by wrapping `Inner`.
  type Output;

  /// Wraps `inner` and returns a new middleware.
  ///
  /// The returned wrapper may conditionally call or skip `inner`,
  /// implementing short-circuit behaviour.
  fn layer(self, inner: Inner) -> Self::Output;
}
