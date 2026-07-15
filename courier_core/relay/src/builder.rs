use std::error::Error;
use std::marker::PhantomData;

use crate::{Chain, Layer, MapErr, MapErrLayer, Middleware, Pipeline, Then};

/// Empty builder state — no middleware added yet.
pub struct Empty;

/// Non-empty builder state — at least one middleware present.
pub struct NonEmpty;

/// A builder for constructing a [`Pipeline`] with a type-state API.
///
/// # States
///
/// | State | Available methods | Semantics |
/// |---|---|---|
/// | [`Empty`] | [`middleware`](PipelineBuilder::middleware),
/// [`then`](PipelineBuilder::then) | Stores the first element |
/// | [`NonEmpty`] | All methods + [`build`](PipelineBuilder::build) |
/// Everything is wrapped in [`Chain`] (or [`Layer`] for
/// `map_err`/`layer`) |
///
/// # Composition
///
/// | Method | Wraps with | Order |
/// |---|---|---|
/// | `.middleware(N)` | [`Chain`] | FIFO |
/// | `.then(F)` | [`Chain`]`<M, `[`Then`]`<F, M::Error>>` | FIFO |
/// | `.layer(L)` | `L::Output` | LIFO |
/// | `.map_err(F)` | [`MapErrLayer`] | LIFO |
pub struct PipelineBuilder<State, M = ()> {
  chain: M,
  _state: PhantomData<State>,
}

// ── Empty state ────────────────────────────────────────────────────

impl PipelineBuilder<Empty> {
  /// Creates a new [`PipelineBuilder`].
  pub fn new() -> Self {
    PipelineBuilder { chain: (), _state: PhantomData }
  }
}

impl Default for PipelineBuilder<Empty> {
  fn default() -> Self {
    Self::new()
  }
}

impl PipelineBuilder<Empty> {
  /// Adds the first middleware to the pipeline.
  ///
  /// Transitions to [`NonEmpty`].
  pub fn middleware<M>(self, m: M) -> PipelineBuilder<NonEmpty, M> {
    PipelineBuilder { chain: m, _state: PhantomData }
  }

  /// Adds a [`Then`] as the first element of the pipeline.
  ///
  /// The closure `f` maps the incoming request to a new type.
  /// Transitions to [`NonEmpty`].
  pub fn then<F, Req, B>(self, f: F) -> PipelineBuilder<NonEmpty, Then<F>>
  where
    Req: Send + 'static,
    F: Fn(Req) -> B + Send + Sync + 'static,
    B: Send + 'static,
  {
    PipelineBuilder { chain: Then::new(f), _state: PhantomData }
  }
}

// ── NonEmpty state ─────────────────────────────────────────────────

impl<M> PipelineBuilder<NonEmpty, M> {
  /// Appends `next` via [`Chain`].
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

  /// Transforms the current chain's response via
  /// [`Chain`]`<M, `[`Then`]`<F, M::Error>>`.
  ///
  /// The current chain runs first, then `f` maps its response. The
  /// [`Then`] borrows `M::Error` so chaining always composes.
  pub fn then<F, B, Req>(
    self, f: F,
  ) -> PipelineBuilder<NonEmpty, Chain<M, Then<F, M::Error>>>
  where
    M: Middleware<Req>,
    F: Fn(M::Response) -> B + Send + Sync + 'static,
    B: Send + 'static,
  {
    let then = Then(f, PhantomData);
    PipelineBuilder { chain: Chain(self.chain, then), _state: PhantomData }
  }

  /// Wraps the current chain with a [`Layer`].
  pub fn layer<L>(self, l: L) -> PipelineBuilder<NonEmpty, L::Output>
  where
    L: Layer<M>,
  {
    PipelineBuilder { chain: l.layer(self.chain), _state: PhantomData }
  }

  /// Wraps the current chain with a [`MapErrLayer`].
  pub fn map_err<F, E2, Req>(
    self, f: F,
  ) -> PipelineBuilder<NonEmpty, MapErr<M, F>>
  where
    M: Middleware<Req>,
    F: Fn(M::Error) -> E2 + Send + Sync + 'static,
    E2: Error + Send + Sync + 'static,
  {
    self.layer(MapErrLayer::new(f))
  }

  /// Finalizes the builder, producing a [`Pipeline`].
  pub fn build(self) -> Pipeline<M> {
    Pipeline { chain: self.chain }
  }
}
