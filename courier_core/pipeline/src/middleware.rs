use std::{error::Error, future::Future};

/// A protocol-agnostic middleware that processes a request.
///
/// Each middleware receives a request of type `Req`, performs a
/// transformation, and returns a [`Response`](Self::Response). There
/// is no explicit signal type — the decision to short-circuit is made
/// by **not** calling the inner middleware.
///
/// # Short-circuit
///
/// A plain middleware that just returns `Ok(...)` will always pass
/// through to the next middleware in a [`Stack`](crate::Stack) chain.
/// To short-circuit, wrap the inner chain with a [`Mixin`](crate::Mixin)
/// and return `Ok(...)` without calling `self.inner.handle(...)`.
///
/// The associated [`Error`] type unifies error handling across the
/// chain.
///
/// # Examples
///
/// ```rust,ignore
/// use std::error::Error;
///
/// use pipeline::Middleware;
///
/// struct Logger;
///
/// impl<Req> Middleware<Req> for Logger {
///   type Response = Req;
///   type Error = Box<dyn Error + Send + Sync>;
///
///   async fn handle(
///     &self, req: Req,
///   ) -> Result<Self::Response, Self::Error> {
///     // log something ...
///     Ok(req)
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
  /// This method always runs when used in a [`Stack`](crate::Stack)
  /// chain. To add short-circuit behaviour, wrap it in a
  /// [`Mixin`](crate::Mixin) whose wrapper holds an inner middleware
  /// and conditionally calls `self.inner.handle(...)`.
  ///
  /// The returned future is `Send` so the middleware can be used across
  /// await points in a multi-threaded runtime.
  fn handle(
    &self, req: Req,
  ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send;
}
