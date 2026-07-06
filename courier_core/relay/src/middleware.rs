use std::{error::Error, future::Future};

/// A protocol-agnostic middleware that processes a request.
///
/// Each middleware receives a request of type `Req`, performs a
/// transformation, and returns a [`Response`](Self::Response).
///
/// # Short-circuit
///
/// A middleware **may** hold an inner middleware and conditionally call it.
/// Short-circuit is an implementation choice: return a response without
/// calling `self.inner.handle(req)` and the remainder of the chain is
/// skipped.
///
/// There is no explicit signal type — the decision to short-circuit is
/// made by the middleware's struct definition, not by the trait.
///
/// When composed via [`Chain`](crate::Chain) (unconditional sequence),
/// both middlewares always run. To add short-circuit behaviour, wrap the
/// chain with a [`Layer`](crate::Layer) whose output owns an inner
/// middleware and may skip it.
///
/// # Examples
///
/// A leaf middleware that always passes through:
///
/// ```rust,ignore
/// use std::error::Error;
///
/// use relay::Middleware;
///
/// struct Logger;
///
/// impl<Req> Middleware<Req> for Logger {
///   type Response = Req;
///   type Error = Box<dyn Error + Send + Sync>;
///
///   async fn handle(
///     &self,
///     req: Req,
///   ) -> Result<Self::Response, Self::Error> {
///     // log something ...
///     Ok(req)
///   }
/// }
/// ```
///
/// A wrapping middleware that can short-circuit:
///
/// ```rust,ignore
/// use std::error::Error;
///
/// use relay::Middleware;
///
/// struct Timeout<Inner> {
///   inner: Inner,
///   duration: std::time::Duration,
/// }
///
/// impl<Req, Inner> Middleware<Req> for Timeout<Inner>
/// where
///   Inner: Middleware<Req>,
/// {
///   type Response = Inner::Response;
///   type Error = Box<dyn Error + Send + Sync>;
///
///   async fn handle(
///     &self,
///     req: Req,
///   ) -> Result<Self::Response, Self::Error> {
///     tokio::time::timeout(
///       self.duration,
///       self.inner.handle(req),
///     )
///     .await
///     .map_err(|_| "timeout".into())? // Short-circuit on timeout
///   }
/// }
/// ```
pub trait Middleware<Req>: Send + Sync + 'static {
  /// The response type produced after processing the request.
  type Response;

  /// The error type returned when this middleware fails.
  type Error: Error + Send + Sync + 'static;

  /// Handles the given request and returns a response or error.
  ///
  /// When used in a [`Chain`](crate::Chain), this method always runs.
  /// When used inside a [`Layer`](crate::Layer) wrapper, the wrapper
  /// may skip calling this method to short-circuit.
  ///
  /// The returned future is `Send` so the middleware can be used across
  /// await points in a multi-threaded runtime.
  fn handle(
    &self, req: Req,
  ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send;
}
