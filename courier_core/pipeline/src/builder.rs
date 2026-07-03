use std::marker::PhantomData;

use crate::{Middleware, Mixin, Pipeline, Stack};

/// A marker type representing the initial state of a
/// [`PipelineBuilder`] when no middleware has been added yet.
#[doc(hidden)]
pub struct Empty;

/// A marker type representing the state of a [`PipelineBuilder`] after
/// at least one middleware has been added.
#[doc(hidden)]
pub struct NonEmpty;

/// A builder for constructing a [`Pipeline`] with a type-state API.
///
/// The builder validates trait bounds at every step, so mismatched
/// request / response or error types are caught at the call site —
/// not buried inside `handle()`.
///
/// # Type parameters
///
/// - `State` — tracks whether a middleware has been added
///   ([`Empty`] / [`NonEmpty`]).
/// - `Req` — the request type accepted by the assembled pipeline.
/// - `M` — the composite middleware type.
///
/// # Composition model
///
/// - **`middleware`** — appends a middleware to the inner chain. All
///   middlewares added this way run unconditionally in sequence. The
///   response of the previous middleware must match the request of the
///   next. Errors flow toward the first middleware via [`Into`].
/// - **`mixin`** — wraps the current chain with a [`Mixin`]. The
///   wrapper may short-circuit by skipping its inner middleware.
///
/// # Examples
///
/// ```rust,ignore
/// use pipeline::PipelineBuilder;
///
/// // handler always runs; AuthMixin wraps it and can short-circuit
/// let pipeline = PipelineBuilder::new()
///   .middleware(handler)
///   .mixin(AuthMixin)
///   .build();
/// ```
pub struct PipelineBuilder<State, Req, M = ()> {
  chain: M,
  _state: PhantomData<(State, Req)>,
}

impl<Req> PipelineBuilder<Empty, Req> {
  /// Creates a new [`PipelineBuilder`] with no middleware.
  ///
  /// The request type `Req` is typically inferred from the first
  /// `.middleware()` call.
  pub fn new() -> Self {
    PipelineBuilder { chain: (), _state: PhantomData }
  }
}

impl<Req> Default for PipelineBuilder<Empty, Req> {
  fn default() -> Self {
    PipelineBuilder::new()
  }
}

impl<Req> PipelineBuilder<Empty, Req> {
  /// Adds the first middleware, transitioning to [`NonEmpty`].
  ///
  /// The request type `Req` is inferred from the middleware's
  /// implementation.
  pub fn middleware<M>(self, m: M) -> PipelineBuilder<NonEmpty, Req, M>
  where
    M: Middleware<Req>,
  {
    PipelineBuilder { chain: m, _state: PhantomData }
  }
}

impl<Req, M> PipelineBuilder<NonEmpty, Req, M>
where
  M: Middleware<Req>,
{
  /// Appends another middleware to the pipeline.
  ///
  /// The new middleware's request type must match the current chain's
  /// response type, and its error type must be convertible into the
  /// chain's error type. Both middlewares always run in sequence.
  ///
  /// Use [`mixin`](Self::mixin) instead if the middleware needs to
  /// conditionally skip the inner chain.
  pub fn middleware<N>(
    self, next: N,
  ) -> PipelineBuilder<NonEmpty, Req, Stack<M, N>>
  where
    N: Middleware<M::Response>,
    N::Error: Into<M::Error>,
  {
    PipelineBuilder { chain: Stack(self.chain, next), _state: PhantomData }
  }

  /// Wraps the current middleware chain with a [`Mixin`].
  ///
  /// The mixin's wrapper runs before the inner middleware and may
  /// short-circuit by not calling `self.inner.handle(req)`.
  /// Multiple mixins can be applied, each wrapping the previous result.
  ///
  /// The chain's error type must be convertible into the wrapper's
  /// error type.
  pub fn mixin<X>(self, mixin: X) -> PipelineBuilder<NonEmpty, Req, X::Wrapper>
  where
    X: Mixin<M>,
    X::Wrapper: Middleware<Req>,
    M::Error: Into<<X::Wrapper as Middleware<Req>>::Error>,
  {
    PipelineBuilder { chain: mixin.mix(self.chain), _state: PhantomData }
  }

  /// Finalizes the builder, producing a [`Pipeline`].
  pub fn build(self) -> Pipeline<M> {
    Pipeline { chain: self.chain }
  }
}
