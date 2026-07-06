use std::error::Error;
use std::marker::PhantomData;

use crate::{Chain, Layer, MapErr, Middleware, Pipeline, Then};

#[doc(hidden)]
pub struct Empty;

#[doc(hidden)]
pub struct NonEmpty;

/// A builder for constructing a [`Pipeline`] with a type-state API.
///
/// The builder validates trait bounds at every step, so mismatched
/// request / response or error types are caught at the call site —
/// not buried inside `handle()`.
///
/// # Composition model
///
/// Two composition patterns are supported, each with a distinct semantic:
///
/// | Method | Order | Dependency |
/// |--------|-------|------------|
/// | `.middleware(N)` | **FIFO** — current runs, then N | Data: N
/// consumes current's output |
/// | `.layer(L)` | **LIFO** — L wraps outermost, runs first | Control: L
/// may skip inner |
///
/// - **`.middleware(N)`** appends a middleware in unconditional sequence.
///   The current chain's response must match N's request. Both always run.
/// - **`.layer(L)`** wraps the current chain with a [`Layer`]. The wrapper
///   runs first and may short-circuit by not calling the inner chain.
///
/// # Examples
///
/// ```rust,ignore
/// use relay::PipelineBuilder;
///
/// // handler always runs; AuthLayer wraps it and can short-circuit
/// let pipeline = PipelineBuilder::new()
///   .middleware(handler)
///   .layer(AuthLayer)
///   .build();
/// ```
pub struct PipelineBuilder<State, M = ()> {
  chain: M,
  _state: PhantomData<State>,
}

impl PipelineBuilder<Empty> {
  /// Creates a new [`PipelineBuilder`] with no middleware.
  pub fn new() -> Self {
    PipelineBuilder { chain: (), _state: PhantomData }
  }
}

impl Default for PipelineBuilder<Empty> {
  fn default() -> Self {
    PipelineBuilder::new()
  }
}

impl PipelineBuilder<Empty> {
  /// Sets the first middleware in the pipeline.
  ///
  /// This is the innermost middleware — it runs last in a sequential
  /// chain and deepest inside layer wrappers. Append more middlewares
  /// with
  /// [`middleware`](Self::middleware) or wrap with
  /// [`layer`](Self::layer).
  pub fn middleware<M>(self, m: M) -> PipelineBuilder<NonEmpty, M> {
    PipelineBuilder { chain: m, _state: PhantomData }
  }
}

impl<M> PipelineBuilder<NonEmpty, M> {
  /// Appends `next` to run **after** the current chain.
  ///
  /// This is unconditional sequential composition (FIFO): the current
  /// chain runs first, its response feeds into `next`. Both always
  /// execute. Errors from `next` must be convertible into the current
  /// chain's error type via [`Into`].
  ///
  /// To conditionally skip the chain, use [`layer`](Self::layer)
  /// instead.
  pub fn middleware<Next, Req>(
    self, next: Next,
  ) -> PipelineBuilder<NonEmpty, Chain<M, Next>>
  where
    M: Middleware<Req>,
    Next: Middleware<M::Response>,
    Next::Error: Into<M::Error>,
  {
    PipelineBuilder { chain: Chain(self.chain, next), _state: PhantomData }
  }

  /// Wraps the current middleware chain with a [`Layer`].
  ///
  /// The layer's output runs **before** the inner chain (LIFO): the
  /// last layer added runs first. The wrapper may short-circuit by
  /// not calling `self.inner.handle(req)`.
  ///
  /// Multiple layers can be applied — each wraps the previous result,
  /// nesting outward.
  pub fn layer<L>(self, l: L) -> PipelineBuilder<NonEmpty, L::Output>
  where
    L: Layer<M>,
  {
    PipelineBuilder { chain: l.layer(self.chain), _state: PhantomData }
  }

  /// Transforms the response type of the current chain.
  ///
  /// The closure `f` maps `M::Response` to `B`, wrapping the chain in a
  /// [`Then`] middleware. Use this when the next middleware or layer
  /// expects a different request type than the current chain produces.
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// let pipeline = PipelineBuilder::new()
  ///   .middleware(parse_int)           // Response = i32
  ///   .then(|n: i32| n.to_string())    // Response = String
  ///   .middleware(handle_string)            // Request = String
  ///   .build();
  /// ```
  pub fn then<F, B, Req>(self, f: F) -> PipelineBuilder<NonEmpty, Then<M, F>>
  where
    M: Middleware<Req>,
    F: Fn(M::Response) -> B + Send + Sync + 'static,
    B: Send + 'static,
  {
    PipelineBuilder {
      chain: Then { inner: self.chain, f },
      _state: PhantomData,
    }
  }

  /// Transforms the error type of the current chain.
  ///
  /// The closure `f` maps `M::Error` to `E2`, wrapping the chain in a
  /// [`MapErr`] middleware. Use this to unify error types across
  /// middleware boundaries.
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// let pipeline = PipelineBuilder::new()
  ///   .middleware(my_service)           // Error = MyError
  ///   .map_err(|e| Box::new(e))         // Error = Box<dyn Error>
  ///   .build();
  /// ```
  pub fn map_err<F, E2, Req>(
    self, f: F,
  ) -> PipelineBuilder<NonEmpty, MapErr<M, F>>
  where
    M: Middleware<Req>,
    F: Fn(M::Error) -> E2 + Send + Sync + 'static,
    E2: Error + Send + Sync + 'static,
  {
    PipelineBuilder {
      chain: MapErr { inner: self.chain, f },
      _state: PhantomData,
    }
  }

  /// Finalizes the builder, producing a [`Pipeline`].
  ///
  /// All type constraints have been validated incrementally during
  /// construction. The returned [`Pipeline`] is ready to handle
  /// requests.
  pub fn build(self) -> Pipeline<M> {
    Pipeline { chain: self.chain }
  }
}
