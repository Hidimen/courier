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
/// | [`Empty`] | [`middleware`](Builder::middleware),
/// [`then`](Builder::then) | Stores the first element |
/// | [`NonEmpty`] | All methods + [`build`](Builder::build) |
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
pub struct Builder<State, M = ()> {
  chain: M,
  _state: PhantomData<State>,
}

impl Builder<Empty> {
  /// Creates a new [`Builder`].
  pub fn new() -> Self {
    Builder { chain: (), _state: PhantomData }
  }
}

impl Default for Builder<Empty> {
  fn default() -> Self {
    Self::new()
  }
}

impl Builder<Empty> {
  /// Adds the first middleware to the pipeline.
  ///
  /// Transitions to [`NonEmpty`].
  pub fn middleware<M>(self, m: M) -> Builder<NonEmpty, M> {
    Builder { chain: m, _state: PhantomData }
  }

  /// Adds a [`Then`] as the first element of the pipeline.
  ///
  /// The closure `f` maps the incoming request to a new type.
  /// Transitions to [`NonEmpty`].
  pub fn then<F, Req, B>(self, f: F) -> Builder<NonEmpty, Then<F>>
  where
    Req: Send + 'static,
    F: Fn(Req) -> B + Send + Sync + 'static,
    B: Send + 'static,
  {
    Builder { chain: Then::new(f), _state: PhantomData }
  }
}

// ── NonEmpty state ─────────────────────────────────────────────────

impl<M> Builder<NonEmpty, M> {
  /// Appends `next` via [`Chain`].
  pub fn middleware<Next, Req>(
    self, next: Next,
  ) -> Builder<NonEmpty, Chain<M, Next>>
  where
    M: Middleware<Req>,
    Next: Middleware<M::Response>,
    Next::Error: Into<M::Error>,
  {
    Builder { chain: Chain(self.chain, next), _state: PhantomData }
  }

  /// Transforms the current chain's response via
  /// [`Chain`]`<M, `[`Then`]`<F, M::Error>>`.
  ///
  /// The current chain runs first, then `f` maps its response. The
  /// [`Then`] borrows `M::Error` so chaining always composes.
  pub fn then<F, B, Req>(
    self, f: F,
  ) -> Builder<NonEmpty, Chain<M, Then<F, M::Error>>>
  where
    M: Middleware<Req>,
    F: Fn(M::Response) -> B + Send + Sync + 'static,
    B: Send + 'static,
  {
    let then = Then(f, PhantomData);
    Builder { chain: Chain(self.chain, then), _state: PhantomData }
  }

  /// Wraps the current chain with a [`Layer`].
  pub fn layer<L>(self, l: L) -> Builder<NonEmpty, L::Output>
  where
    L: Layer<M>,
  {
    Builder { chain: l.layer(self.chain), _state: PhantomData }
  }

  /// Wraps the current chain with a [`MapErrLayer`].
  pub fn map_err<F, E2, Req>(self, f: F) -> Builder<NonEmpty, MapErr<M, F>>
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
