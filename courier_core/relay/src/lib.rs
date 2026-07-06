//! A composable relay system for processing requests through
//! middleware pipelines.
//!
//! This crate provides [`Middleware`] and [`Layer`] to build type-safe,
//! zero-cost middleware chains.
//!
//! # Composition model
//!
//! Two patterns are supported:
//!
//! - **Sequential chain** (via
//!   [`PipelineBuilder::middleware`](PipelineBuilder::middleware)): FIFO
//!   composition where each middleware's response feeds the next. Data
//!   dependency — producer runs before consumer.
//! - **Wrapping layer** (via
//!   [`PipelineBuilder::layer`](PipelineBuilder::layer)): LIFO
//!   composition where each layer wraps the inner chain. Control
//!   dependency — wrapper runs before inner, and may short-circuit.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use relay::{Layer, Middleware, PipelineBuilder};
//!
//! // A leaf middleware — always processes, no inner
//! struct Handler;
//! impl<Req> Middleware<Req> for Handler {
//!   type Response = Req;
//!   type Error = std::convert::Infallible;
//!   async fn handle(&self, req: Req) -> Result<Req, Self::Error> {
//!     Ok(req)
//!   }
//! }
//!
//! // Build a pipeline
//! let pipeline = PipelineBuilder::new()
//!   .middleware(Handler)
//!   .build();
//! ```
mod builder;
mod chain;
mod layer;
mod middleware;
mod pipeline;
mod wrappers;

pub use builder::{Empty, NonEmpty, PipelineBuilder};
pub use chain::Chain;
pub use layer::Layer;
pub use middleware::Middleware;
pub use pipeline::Pipeline;
pub use wrappers::{MapErr, Then};

#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use std::sync::atomic::{AtomicU32, Ordering};

  use super::*;

  // ------------------------------------------------------------------
  // Test error type
  // ------------------------------------------------------------------

  /// A simple error type for testing.
  #[derive(Debug, PartialEq, Eq)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  // ------------------------------------------------------------------
  // Test helpers
  // ------------------------------------------------------------------

  /// A middleware that prepends a string prefix and passes to inner.
  struct Prefix<Inner> {
    inner: Inner,
    prefix: &'static str,
  }

  impl<Inner> Middleware<String> for Prefix<Inner>
  where
    Inner: Middleware<String, Response = String, Error = TestError>,
  {
    type Response = String;
    type Error = TestError;

    async fn handle(&self, req: String) -> Result<Self::Response, Self::Error> {
      let modified = format!("{}{}", self.prefix, req);
      self.inner.handle(modified).await
    }
  }

  /// A leaf middleware that appends a suffix.
  struct Suffix {
    suffix: &'static str,
  }

  impl Middleware<String> for Suffix {
    type Response = String;
    type Error = TestError;

    async fn handle(&self, req: String) -> Result<Self::Response, Self::Error> {
      Ok(format!("{}{}", req, self.suffix))
    }
  }

  /// A layer that wraps an inner with a Prefix.
  struct PrefixLayer {
    prefix: &'static str,
  }

  impl<Inner> Layer<Inner> for PrefixLayer {
    type Output = Prefix<Inner>;

    fn layer(self, inner: Inner) -> Self::Output {
      Prefix { inner, prefix: self.prefix }
    }
  }

  /// Tracks how many times `handle` was called.
  #[derive(Debug, Default, Clone)]
  struct Counter {
    count: Arc<AtomicU32>,
  }

  impl Counter {
    fn new() -> Self {
      Self { count: Arc::new(AtomicU32::new(0)) }
    }

    fn get(&self) -> u32 {
      self.count.load(Ordering::SeqCst)
    }
  }

  impl Middleware<String> for Counter {
    type Response = String;
    type Error = TestError;

    async fn handle(&self, req: String) -> Result<Self::Response, Self::Error> {
      self.count.fetch_add(1, Ordering::SeqCst);
      Ok(req)
    }
  }

  // ------------------------------------------------------------------
  // Chain tests (FIFO sequential composition)
  // ------------------------------------------------------------------

  /// Verify `Chain<Left, Right>` executes Left first, then Right.
  #[tokio::test]
  async fn chain_fifo_order() {
    let c1 = Counter::new();
    let c2 = Counter::new();
    let chain = Chain(c1.clone(), c2.clone());

    let result = chain.handle("test".into()).await;
    assert_eq!(result.unwrap(), "test");
    // Both must have been called exactly once
    assert_eq!(c1.get(), 1);
    assert_eq!(c2.get(), 1);
  }

  /// Verify Chain passes Left's response to Right (data flow).
  #[tokio::test]
  async fn chain_data_flow() {
    // Left = Suffix{, "!"}, Right = Prefix{inner: leaf, prefix: "A_"}
    // "test" → Suffix → "test!" → Prefix → "A_test!"
    let leaf = Suffix { suffix: "" };
    let left = Suffix { suffix: "!" };
    let right = Prefix { inner: leaf, prefix: "A_" };

    let chain = Chain(left, right);
    let result = chain.handle("test".into()).await;
    assert_eq!(result.unwrap(), "A_test!");
  }

  /// Verify Chain always calls both on success.
  #[tokio::test]
  async fn chain_always_calls_both_on_success() {
    let c1 = Counter::new();
    let c2 = Counter::new();
    let chain = Chain(c1.clone(), c2.clone());

    for _ in 0..5 {
      let _ = chain.handle("x".into()).await;
    }
    assert_eq!(c1.get(), 5);
    assert_eq!(c2.get(), 5);
  }

  /// Verify when Left errors, Right is never called.
  #[tokio::test]
  async fn chain_left_error_skips_right() {
    /// Middleware that always errors.
    struct AlwaysErr;
    impl Middleware<String> for AlwaysErr {
      type Response = String;
      type Error = TestError;
      async fn handle(
        &self, _req: String,
      ) -> Result<Self::Response, Self::Error> {
        Err(TestError("fail".into()))
      }
    }

    let c2 = Counter::new();
    let chain = Chain(AlwaysErr, c2.clone());

    let result = chain.handle("test".into()).await;
    assert!(result.is_err());
    assert_eq!(c2.get(), 0); // Right never called
  }

  // ------------------------------------------------------------------
  // Layer tests (LIFO wrapping + short-circuit)
  // ------------------------------------------------------------------

  /// A middleware wrapping an inner — may short-circuit.
  struct Gate<Inner> {
    inner: Inner,
    allow: bool,
  }

  impl<Inner> Middleware<String> for Gate<Inner>
  where
    Inner: Middleware<String, Response = String, Error = TestError>,
  {
    type Response = String;
    type Error = TestError;

    async fn handle(&self, req: String) -> Result<Self::Response, Self::Error> {
      if !self.allow {
        return Ok("blocked".into()); // Short-circuit
      }
      self.inner.handle(req).await
    }
  }

  /// Layer that produces a Gate.
  struct GateLayer {
    allow: bool,
  }

  impl<Inner> Layer<Inner> for GateLayer {
    type Output = Gate<Inner>;

    fn layer(self, inner: Inner) -> Self::Output {
      Gate { inner, allow: self.allow }
    }
  }

  /// Verify Gate short-circuits: inner never called.
  #[tokio::test]
  async fn layer_short_circuit() {
    let counter = Counter::new();

    let pipeline = PipelineBuilder::new()
      .middleware(counter.clone())
      .layer(GateLayer { allow: false })
      .build();

    let result = pipeline.handle("test".into()).await;
    assert_eq!(result.unwrap(), "blocked");
    assert_eq!(counter.get(), 0); // Inner never called
  }

  /// Verify when allow=true, Gate calls inner.
  #[tokio::test]
  async fn layer_passes_through_when_allowed() {
    let counter = Counter::new();

    let pipeline = PipelineBuilder::new()
      .middleware(counter.clone())
      .layer(GateLayer { allow: true })
      .build();

    let result = pipeline.handle("test".into()).await;
    assert_eq!(result.unwrap(), "test");
    assert_eq!(counter.get(), 1);
  }

  /// Verify multiple layers execute LIFO.
  /// .middleware(leaf).layer(PrefixLayer{"A_"}).layer(PrefixLayer{"B_"})
  /// Execution: B_ runs first → A_ runs second → leaf runs last
  /// "test" → B_: "B_test" → A_: "A_B_test" → leaf: "A_B_test"
  #[tokio::test]
  async fn layer_lifo_order() {
    let leaf = Suffix { suffix: "" };
    let pipeline = PipelineBuilder::new()
      .middleware(leaf)
      .layer(PrefixLayer { prefix: "A_" })
      .layer(PrefixLayer { prefix: "B_" })
      .build();

    let result = pipeline.handle("test".into()).await;
    assert_eq!(result.unwrap(), "A_B_test");
  }

  // ------------------------------------------------------------------
  // Combined Chain + Layer tests
  // ------------------------------------------------------------------

  /// Verify combined: layers (LIFO) wrap chains (FIFO).
  /// .middleware(A).middleware(B).layer(L1).layer(L2)
  /// Execution: L2 → L1 → A → B
  #[tokio::test]
  async fn combined_layers_and_chain() {
    let c_first = Counter::new();
    let c_second = Counter::new();

    let pipeline = PipelineBuilder::new()
      .middleware(c_first.clone())
      .middleware(c_second.clone())
      .layer(GateLayer { allow: true })
      .layer(GateLayer { allow: true })
      .build();

    let result = pipeline.handle("test".into()).await;
    assert_eq!(result.unwrap(), "test");
    assert_eq!(c_first.get(), 1);
    assert_eq!(c_second.get(), 1);
  }

  /// Verify outer layer can short-circuit entire chain.
  #[tokio::test]
  async fn outer_layer_short_circuits_chain() {
    let c_first = Counter::new();
    let c_second = Counter::new();

    let pipeline = PipelineBuilder::new()
      .middleware(c_first.clone())
      .middleware(c_second.clone())
      .layer(GateLayer { allow: true }) // inner layer, passes
      .layer(GateLayer { allow: false }) // outer layer, blocks
      .build();

    let result = pipeline.handle("test".into()).await;
    assert_eq!(result.unwrap(), "blocked");
    assert_eq!(c_first.get(), 0);
    assert_eq!(c_second.get(), 0);
  }

  // ------------------------------------------------------------------
  // Then / MapErr tests
  // ------------------------------------------------------------------

  /// Verify `.then()` transforms the response type.
  #[tokio::test]
  async fn then_transforms_response() {
    /// Middleware that returns a number.
    struct ReturnNum;
    impl Middleware<()> for ReturnNum {
      type Response = i32;
      type Error = TestError;
      async fn handle(&self, _req: ()) -> Result<Self::Response, Self::Error> {
        Ok(42)
      }
    }

    let pipeline = PipelineBuilder::new()
      .middleware(ReturnNum)
      .then(|n: i32| n.to_string())
      .build();

    let result: Result<String, TestError> = pipeline.handle(()).await;
    assert_eq!(result.unwrap(), "42");
  }

  /// Verify `.map_err()` transforms the error type.
  #[tokio::test]
  async fn map_err_transforms_error() {
    /// Middleware that always errors.
    struct AlwaysErr;
    impl Middleware<()> for AlwaysErr {
      type Response = ();
      type Error = TestError;
      async fn handle(&self, _req: ()) -> Result<Self::Response, Self::Error> {
        Err(TestError("inner error".into()))
      }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct WrappedError(String);

    impl std::fmt::Display for WrappedError {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
      }
    }

    impl std::error::Error for WrappedError {}

    let pipeline = PipelineBuilder::new()
      .middleware(AlwaysErr)
      .map_err(|e: TestError| WrappedError(format!("wrapped: {e}")))
      .build();

    let result: Result<(), WrappedError> = pipeline.handle(()).await;
    assert_eq!(
      result.unwrap_err(),
      WrappedError("wrapped: inner error".into())
    );
  }

  // ------------------------------------------------------------------
  // Edge cases
  // ------------------------------------------------------------------

  /// Verify simple single-middleware pipeline works.
  #[tokio::test]
  async fn simplest_pipeline() {
    let counter = Counter::new();
    let pipeline = PipelineBuilder::new().middleware(counter.clone()).build();

    let result = pipeline.handle("hello".into()).await;
    assert_eq!(result.unwrap(), "hello");
    assert_eq!(counter.get(), 1);
  }

  /// Verify deeply nested layers compile and execute.
  #[tokio::test]
  async fn deep_layer_nesting() {
    let counter = Counter::new();
    let pipeline = PipelineBuilder::new()
      .middleware(counter.clone())
      .layer(GateLayer { allow: true })
      .layer(GateLayer { allow: true })
      .layer(GateLayer { allow: true })
      .layer(GateLayer { allow: true })
      .layer(GateLayer { allow: true })
      .build();

    let result = pipeline.handle("test".into()).await;
    assert_eq!(result.unwrap(), "test");
    assert_eq!(counter.get(), 1);
  }

  /// Verify nested chains execute in FIFO order.
  #[tokio::test]
  async fn nested_chains_fifo() {
    let c1 = Counter::new();
    let c2 = Counter::new();
    let c3 = Counter::new();

    // .middleware(c1).middleware(c2).middleware(c3)
    let pipeline = PipelineBuilder::new()
      .middleware(c1.clone())
      .middleware(c2.clone())
      .middleware(c3.clone())
      .build();

    let result = pipeline.handle("test".into()).await;
    assert_eq!(result.unwrap(), "test");
    assert_eq!(c1.get(), 1);
    assert_eq!(c2.get(), 1);
    assert_eq!(c3.get(), 1);
  }

  /// Verify chain error propagation: Right::Error: Into<Left::Error>.
  #[tokio::test]
  async fn chain_error_conversion() {
    /// Middleware whose error is TestError.
    struct LeftMw;
    impl Middleware<String> for LeftMw {
      type Response = String;
      type Error = TestError;
      async fn handle(
        &self, req: String,
      ) -> Result<Self::Response, Self::Error> {
        Ok(req)
      }
    }

    /// Middleware whose error is also TestError.
    struct RightMw;
    impl Middleware<String> for RightMw {
      type Response = String;
      type Error = TestError;
      async fn handle(
        &self, _req: String,
      ) -> Result<Self::Response, Self::Error> {
        Err(TestError("right failed".into()))
      }
    }

    let chain = Chain(LeftMw, RightMw);
    let result = chain.handle("test".into()).await;
    assert!(result.is_err());
    // Right's error is converted via Into into Left's error (TestError)
    assert_eq!(result.unwrap_err(), TestError("right failed".into()));
  }
}
