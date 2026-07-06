use std::sync::Arc;

use logger::Logger;
use network::transport::Transport;
use relay::Pipeline;

use crate::{Depot, error::DepotBuildError};

/// A builder for constructing a [`Depot`].
///
/// All four fields — transport, protocol, pipeline, and logger — are
/// required. Each setter method infers the corresponding type parameter
/// from the argument, so no explicit type annotations are needed.
///
/// # Type parameters
///
/// - `T` — the transport type (defaults to `()` when unset).
/// - `P` — the protocol type (defaults to `()` when unset).
/// - `M` — the middleware type (defaults to `()` when unset).
///
/// # Examples
///
/// ```rust,ignore
/// use depot::DepotBuilder;
///
/// let depot = DepotBuilder::new()
///     .transport(my_transport)
///     .protocol(my_protocol)
///     .pipeline(my_pipeline)
///     .logger(my_logger)
///     .build()
///     .expect("all required fields set");
/// ```
#[must_use = "builder methods consume self and return a new builder"]
pub struct DepotBuilder<T = (), P = (), M = ()> {
  transport: Option<T>,
  protocol: Option<P>,
  pipeline: Option<Pipeline<M>>,
  logger: Option<Arc<Logger>>,
}

impl DepotBuilder {
  /// Creates a new [`DepotBuilder`] with no fields set.
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// use depot::DepotBuilder;
  ///
  /// let builder = DepotBuilder::new();
  /// ```
  pub fn new() -> Self {
    Self { transport: None, protocol: None, pipeline: None, logger: None }
  }
}

impl Default for DepotBuilder {
  /// Creates a default [`DepotBuilder`] with no fields set.
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Transport, P, M> DepotBuilder<T, P, M> {
  /// Sets the protocol for the [`Depot`], inferring `P` from the
  /// argument.
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// use depot::DepotBuilder;
  ///
  /// let builder = DepotBuilder::new()
  ///     .protocol(my_protocol);
  /// ```
  pub fn protocol<P2>(self, protocol: P2) -> DepotBuilder<T, P2, M> {
    DepotBuilder {
      transport: self.transport,
      protocol: Some(protocol),
      pipeline: self.pipeline,
      logger: self.logger,
    }
  }
}

impl<T, P, M> DepotBuilder<T, P, M> {
  /// Sets the transport for the [`Depot`], inferring `T` from the
  /// argument.
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// use depot::DepotBuilder;
  ///
  /// let builder = DepotBuilder::new()
  ///     .transport(my_transport);
  /// ```
  pub fn transport<T2>(self, transport: T2) -> DepotBuilder<T2, P, M> {
    DepotBuilder {
      transport: Some(transport),
      protocol: self.protocol,
      pipeline: self.pipeline,
      logger: self.logger,
    }
  }

  /// Sets the pipeline for the [`Depot`], inferring `M` from the
  /// argument.
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// use depot::DepotBuilder;
  ///
  /// let builder = DepotBuilder::new()
  ///     .pipeline(my_pipeline);
  /// ```
  pub fn pipeline<M2>(self, pipeline: Pipeline<M2>) -> DepotBuilder<T, P, M2> {
    DepotBuilder {
      transport: self.transport,
      protocol: self.protocol,
      pipeline: Some(pipeline),
      logger: self.logger,
    }
  }

  /// Sets the logger for the [`Depot`].
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// use depot::DepotBuilder;
  ///
  /// let builder = DepotBuilder::new()
  ///     .logger(my_logger);
  /// ```
  pub fn logger(mut self, logger: Arc<Logger>) -> Self {
    self.logger = Some(logger);
    self
  }

  /// Consumes the builder and produces a [`Depot`].
  ///
  /// # Errors
  ///
  /// Returns [`DepotBuildError`] if any required field — transport,
  /// protocol, pipeline, or logger — has not been set.
  ///
  /// # Examples
  ///
  /// ```rust,ignore
  /// use depot::DepotBuilder;
  ///
  /// let depot = DepotBuilder::new()
  ///     .transport(my_transport)
  ///     .protocol(my_protocol)
  ///     .pipeline(my_pipeline)
  ///     .logger(my_logger)
  ///     .build()
  ///     .expect("all required fields set");
  /// ```
  pub fn build(self) -> Result<Depot<T, P, M>, DepotBuildError> {
    Ok(Depot::new(
      self.transport.ok_or(DepotBuildError::MissingTransport)?,
      self.protocol.ok_or(DepotBuildError::MissingProtocol)?,
      self.pipeline.ok_or(DepotBuildError::MissingPipeline)?,
      self.logger.ok_or(DepotBuildError::MissingLogger)?,
    ))
  }
}

#[cfg(test)]
mod tests {
  use std::{net::SocketAddr, str::FromStr};

  use super::*;

  /// A stub transport for testing.
  #[derive(Debug)]
  struct StubTransport;

  impl Transport for StubTransport {
    fn bind<A: std::net::ToSocketAddrs + Send>(
      _addr: A,
    ) -> std::io::Result<Self> {
      Ok(Self)
    }

    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
      Ok(SocketAddr::from_str("0.0.0.0:8080").unwrap())
    }

    fn set_ttl(&self, _ttl: u32) -> std::io::Result<()> {
      Ok(())
    }

    fn ttl(&self) -> std::io::Result<u32> {
      Ok(100)
    }
  }

  /// A stub protocol for testing.
  #[derive(Debug)]
  struct StubProtocol;

  /// A stub error for testing.
  #[derive(Debug)]
  struct StubError;

  impl std::fmt::Display for StubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "stub error")
    }
  }

  impl std::error::Error for StubError {}

  /// A stub middleware for testing.
  #[derive(Debug)]
  struct StubMiddleware;

  impl relay::Middleware<()> for StubMiddleware {
    type Response = ();
    type Error = StubError;

    async fn handle(&self, req: ()) -> Result<Self::Response, Self::Error> {
      Ok(req)
    }
  }

  /// A stub logger for testing.
  fn stub_logger() -> Arc<Logger> {
    use logger::{Builder, Level};
    Builder::new()
      .capacity(128)
      .add_flow(logger::flows::ConsoleFlow::new(Level::Trace))
      .build_local()
      .expect("stub logger")
  }

  /// A stub pipeline for testing.
  fn stub_pipeline() -> Pipeline<StubMiddleware> {
    use relay::PipelineBuilder;
    PipelineBuilder::new().middleware(StubMiddleware).build()
  }

  #[test]
  fn build_with_all_fields_succeeds() {
    let depot = DepotBuilder::new()
      .transport(StubTransport)
      .protocol(StubProtocol)
      .pipeline(stub_pipeline())
      .logger(stub_logger())
      .build();

    assert!(depot.is_ok());
  }

  #[test]
  fn build_without_protocol_fails() {
    let result = DepotBuilder::new()
      .transport(StubTransport)
      .pipeline(stub_pipeline())
      .logger(stub_logger())
      .build();

    match result {
      Err(e) => assert!(e.to_string().contains("protocol")),
      Ok(_) => panic!("expected build to fail without protocol"),
    }
  }

  #[test]
  fn build_without_pipeline_fails() {
    let result = DepotBuilder::new()
      .transport(StubTransport)
      .protocol(StubProtocol)
      .logger(stub_logger())
      .build();

    match result {
      Err(e) => assert!(e.to_string().contains("pipeline")),
      Ok(_) => panic!("expected build to fail without pipeline"),
    }
  }

  #[test]
  fn build_without_logger_fails() {
    let result = DepotBuilder::new()
      .transport(StubTransport)
      .protocol(StubProtocol)
      .pipeline(stub_pipeline())
      .build();

    match result {
      Err(e) => assert!(e.to_string().contains("logger")),
      Ok(_) => panic!("expected build to fail without logger"),
    }
  }

  #[test]
  fn default_builder_is_empty() {
    let _builder = DepotBuilder::default();
  }
}
