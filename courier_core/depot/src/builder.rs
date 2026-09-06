use std::sync::Arc;

use event::Dispatcher;
use logger::Logger;
use network::transport::Transport;
use relay::Pipeline;
use tokio_util::sync::CancellationToken;

use crate::{Depot, error::BuildError};

/// A builder for constructing a [`Depot`].
///
/// # Examples
///
/// ```rust
/// use depot::Builder;
/// use std::sync::Arc;
///
/// use event::Dispatcher;
///
/// # use logger::{Builder as LoggerBuilder, Level, Logger};
/// # use network::transport::Transport;
/// # use relay::{Builder as RelayBuilder, Middleware, Pipeline};
/// #
/// # // Minimal stubs so the example runs standalone.
/// # struct MyTransport;
/// # impl Transport for MyTransport {
/// #   fn bind<A: std::net::ToSocketAddrs + Send>(_addr: A) -> std::io::Result<Self> {
/// #     Ok(Self)
/// #   }
/// #   fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
/// #     Ok("127.0.0.1:8080".parse().unwrap())
/// #   }
/// #   fn set_ttl(&self, _ttl: u32) -> std::io::Result<()> {
/// #     Ok(())
/// #   }
/// #   fn ttl(&self) -> std::io::Result<u32> {
/// #     Ok(100)
/// #   }
/// # }
/// # struct MyProtocol;
/// # struct MyMiddleware;
/// # impl Middleware<()> for MyMiddleware {
/// #   type Response = ();
/// #   type Error = std::convert::Infallible;
/// #   async fn handle(&self, _req: ()) -> Result<Self::Response, Self::Error> {
/// #     Ok(())
/// #   }
/// # }
/// # fn my_pipeline() -> Pipeline<MyMiddleware> {
/// #   RelayBuilder::new().middleware(MyMiddleware).build()
/// # }
/// # fn my_logger() -> Arc<Logger> {
/// #   LoggerBuilder::new()
/// #     .capacity(128)
/// #     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
/// #     .build_local()
/// #     .expect("logger")
/// # }
///
/// let depot = Builder::new()
///     .transport(MyTransport)
///     .protocol(MyProtocol)
///     .pipeline(my_pipeline())
///     .logger(my_logger())
///     .dispatcher(Arc::new(Dispatcher::default()))
///     .build()
///     .expect("all required fields set");
/// ```
#[must_use = "builder methods consume self and return a new builder"]
pub struct Builder<T = (), P = (), M = ()> {
  transport: Option<T>,
  protocol: Option<P>,
  pipeline: Option<Pipeline<M>>,
  logger: Option<Arc<Logger>>,
  dispatcher: Option<Arc<Dispatcher>>,
  cancellation_token: Option<Arc<CancellationToken>>,
}

impl Builder {
  /// Creates a new [`Builder`] with no fields set.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use depot::Builder;
  ///
  /// let builder = Builder::new();
  /// ```
  pub fn new() -> Self {
    Self {
      transport: None,
      protocol: None,
      pipeline: None,
      logger: None,
      dispatcher: None,
      cancellation_token: None,
    }
  }
}

impl Default for Builder {
  /// Creates a default [`Builder`] with no fields set.
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Transport, P, M> Builder<T, P, M> {
  /// Sets the protocol for the [`Depot`], inferring `P` from the
  /// argument.
  ///
  /// Note that a transport must already be set — the `protocol`
  /// method is only available once the builder has a [`Transport`].
  ///
  /// # Examples
  ///
  /// ```rust
  /// use depot::Builder;
  ///
  /// # struct MyTransport;
  /// # impl network::transport::Transport for MyTransport {
  /// #   fn bind<A: std::net::ToSocketAddrs + Send>(_addr: A) -> std::io::Result<Self> { Ok(Self) }
  /// #   fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> { Ok("127.0.0.1:8080".parse().unwrap()) }
  /// #   fn set_ttl(&self, _ttl: u32) -> std::io::Result<()> { Ok(()) }
  /// #   fn ttl(&self) -> std::io::Result<u32> { Ok(100) }
  /// # }
  /// # struct MyProtocol;
  ///
  /// let builder = Builder::new()
  ///     .transport(MyTransport)
  ///     .protocol(MyProtocol);
  /// ```
  pub fn protocol<P2>(self, protocol: P2) -> Builder<T, P2, M> {
    Builder {
      transport: self.transport,
      protocol: Some(protocol),
      pipeline: self.pipeline,
      logger: self.logger,
      dispatcher: self.dispatcher,
      cancellation_token: self.cancellation_token,
    }
  }
}

impl<T, P, M> Builder<T, P, M> {
  /// Sets the transport for the [`Depot`], inferring `T` from the
  /// argument.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use depot::Builder;
  ///
  /// # struct MyTransport;
  /// # impl network::transport::Transport for MyTransport {
  /// #   fn bind<A: std::net::ToSocketAddrs + Send>(_addr: A) -> std::io::Result<Self> { Ok(Self) }
  /// #   fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> { Ok("127.0.0.1:8080".parse().unwrap()) }
  /// #   fn set_ttl(&self, _ttl: u32) -> std::io::Result<()> { Ok(()) }
  /// #   fn ttl(&self) -> std::io::Result<u32> { Ok(100) }
  /// # }
  ///
  /// let builder = Builder::new()
  ///     .transport(MyTransport);
  /// ```
  pub fn transport<T2>(self, transport: T2) -> Builder<T2, P, M> {
    Builder {
      transport: Some(transport),
      protocol: self.protocol,
      pipeline: self.pipeline,
      logger: self.logger,
      dispatcher: self.dispatcher,
      cancellation_token: self.cancellation_token,
    }
  }

  /// Sets the pipeline for the [`Depot`], inferring `M` from the
  /// argument.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use depot::Builder;
  ///
  /// # use relay::{Builder as RelayBuilder, Middleware, Pipeline};
  /// # struct MyMiddleware;
  /// # impl Middleware<()> for MyMiddleware {
  /// #   type Response = ();
  /// #   type Error = std::convert::Infallible;
  /// #   async fn handle(&self, _req: ()) -> Result<Self::Response, Self::Error> { Ok(()) }
  /// # }
  /// # fn my_pipeline() -> Pipeline<MyMiddleware> {
  /// #   RelayBuilder::new().middleware(MyMiddleware).build()
  /// # }
  ///
  /// let builder = Builder::new()
  ///     .pipeline(my_pipeline());
  /// ```
  pub fn pipeline<M2>(self, pipeline: Pipeline<M2>) -> Builder<T, P, M2> {
    Builder {
      transport: self.transport,
      protocol: self.protocol,
      pipeline: Some(pipeline),
      logger: self.logger,
      dispatcher: self.dispatcher,
      cancellation_token: self.cancellation_token,
    }
  }

  /// Sets the logger for the [`Depot`].
  ///
  /// # Examples
  ///
  /// ```rust
  /// use depot::Builder;
  /// use std::sync::Arc;
  ///
  /// # use logger::{Builder as LoggerBuilder, Level, Logger};
  /// # fn my_logger() -> Arc<Logger> {
  /// #   LoggerBuilder::new()
  /// #     .capacity(128)
  /// #     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  /// #     .build_local()
  /// #     .expect("logger")
  /// # }
  ///
  /// let builder = Builder::new()
  ///     .logger(my_logger());
  /// ```
  pub fn logger(mut self, logger: Arc<Logger>) -> Self {
    self.logger = Some(logger);
    self
  }

  /// Sets the dispatcher for the [`Depot`].
  ///
  /// # Examples
  ///
  /// ```rust
  /// use depot::Builder;
  /// use std::sync::Arc;
  ///
  /// use event::Dispatcher;
  ///
  /// let builder = Builder::new()
  ///     .dispatcher(Arc::new(Dispatcher::default()));
  /// ```
  pub fn dispatcher(mut self, dispatcher: Arc<Dispatcher>) -> Self {
    self.dispatcher = Some(dispatcher);
    self
  }

  /// Sets the [`CancellationToken`] that controls the [`Depot`] shutdown.
  ///
  /// When the token is cancelled, the server stops accepting new
  /// connections and waits for the remaining tasks to finish. If not set,
  /// the [`Depot`] creates its own token.
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::sync::Arc;
  /// # use tokio_util::sync::CancellationToken;
  /// use depot::Builder;
  ///
  /// let builder = Builder::new()
  ///     .cancellation_token(Arc::new(CancellationToken::new()));
  /// ```
  pub fn cancellation_token(
    mut self, cancellation_token: Arc<CancellationToken>,
  ) -> Self {
    self.cancellation_token = Some(cancellation_token);
    self
  }

  /// Consumes the builder and produces a [`Depot`].
  ///
  /// # Errors
  ///
  /// Returns [`BuildError`] if any required field — transport,
  /// protocol, pipeline, logger, or event bus — has not been set.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use depot::Builder;
  /// use std::sync::Arc;
  ///
  /// use event::Dispatcher;
  ///
  /// # use logger::{Builder as LoggerBuilder, Level, Logger};
  /// # use network::transport::Transport;
  /// # use relay::{Builder as RelayBuilder, Middleware, Pipeline};
  /// #
  /// # struct MyTransport;
  /// # impl Transport for MyTransport {
  /// #   fn bind<A: std::net::ToSocketAddrs + Send>(_addr: A) -> std::io::Result<Self> { Ok(Self) }
  /// #   fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> { Ok("127.0.0.1:8080".parse().unwrap()) }
  /// #   fn set_ttl(&self, _ttl: u32) -> std::io::Result<()> { Ok(()) }
  /// #   fn ttl(&self) -> std::io::Result<u32> { Ok(100) }
  /// # }
  /// # struct MyProtocol;
  /// # struct MyMiddleware;
  /// # impl Middleware<()> for MyMiddleware {
  /// #   type Response = ();
  /// #   type Error = std::convert::Infallible;
  /// #   async fn handle(&self, _req: ()) -> Result<Self::Response, Self::Error> { Ok(()) }
  /// # }
  /// # fn my_pipeline() -> Pipeline<MyMiddleware> {
  /// #   RelayBuilder::new().middleware(MyMiddleware).build()
  /// # }
  /// # fn my_logger() -> Arc<Logger> {
  /// #   LoggerBuilder::new()
  /// #     .capacity(128)
  /// #     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  /// #     .build_local()
  /// #     .expect("logger")
  /// # }
  ///
  /// let depot = Builder::new()
  ///     .transport(MyTransport)
  ///     .protocol(MyProtocol)
  ///     .pipeline(my_pipeline())
  ///     .logger(my_logger())
  ///     .dispatcher(Arc::new(Dispatcher::default()))
  ///     .build()
  ///     .expect("all required fields set");
  ///
  /// // Omitting a required field fails the build.
  /// let result = Builder::new().transport(MyTransport).build();
  /// assert!(result.is_err());
  /// ```
  pub fn build(self) -> Result<Depot<T, P, M>, BuildError> {
    Ok(Depot::new(
      self.transport.ok_or(BuildError::MissingTransport)?,
      self.protocol.ok_or(BuildError::MissingProtocol)?,
      self.pipeline.ok_or(BuildError::MissingPipeline)?,
      self.logger.ok_or(BuildError::MissingLogger)?,
      self.dispatcher.ok_or(BuildError::MissingDispatcher)?,
      self
        .cancellation_token
        .unwrap_or_else(|| Arc::new(CancellationToken::new())),
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
    use relay::Builder;
    Builder::new().middleware(StubMiddleware).build()
  }

  /// A stub event bus for testing.
  fn stub_dispatcher() -> Arc<Dispatcher> {
    use event::Dispatcher;

    Arc::new(Dispatcher::default())
  }

  #[test]
  fn build_with_all_fields_succeeds() {
    let depot = Builder::new()
      .transport(StubTransport)
      .protocol(StubProtocol)
      .pipeline(stub_pipeline())
      .logger(stub_logger())
      .dispatcher(stub_dispatcher())
      .build();

    assert!(depot.is_ok());
  }

  #[test]
  fn build_without_protocol_fails() {
    let result = Builder::new()
      .transport(StubTransport)
      .pipeline(stub_pipeline())
      .logger(stub_logger())
      .dispatcher(stub_dispatcher())
      .build();

    match result {
      Err(e) => assert!(e.to_string().contains("protocol")),
      Ok(_) => panic!("expected build to fail without protocol"),
    }
  }

  #[test]
  fn build_without_pipeline_fails() {
    let result = Builder::new()
      .transport(StubTransport)
      .protocol(StubProtocol)
      .logger(stub_logger())
      .dispatcher(stub_dispatcher())
      .build();

    match result {
      Err(e) => assert!(e.to_string().contains("pipeline")),
      Ok(_) => panic!("expected build to fail without pipeline"),
    }
  }

  #[test]
  fn build_without_logger_fails() {
    let result = Builder::new()
      .transport(StubTransport)
      .protocol(StubProtocol)
      .pipeline(stub_pipeline())
      .dispatcher(stub_dispatcher())
      .build();

    match result {
      Err(e) => assert!(e.to_string().contains("logger")),
      Ok(_) => panic!("expected build to fail without logger"),
    }
  }

  #[test]
  fn build_without_dispatcher_fails() {
    let result = Builder::new()
      .transport(StubTransport)
      .protocol(StubProtocol)
      .logger(stub_logger())
      .pipeline(stub_pipeline())
      .build();

    match result {
      Err(e) => assert!(e.to_string().contains("dispatcher")),
      Ok(_) => panic!("expected build to fail without logger"),
    }
  }

  #[test]
  fn default_builder_is_empty() {
    let _builder = Builder::default();
  }
}
