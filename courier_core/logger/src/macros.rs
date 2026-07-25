/// Logs a message at the [`Trace`](crate::Level::Trace) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed via
/// [`Builder::build`](crate::Builder::build).
///
/// # Usage
///
/// ```rust,no_run
/// # use logger::trace;
/// # // Requires a globally-installed logger.
/// # let val = 42;
/// trace!("Trace message");
/// trace!("x = {}", val);
/// trace!("my_mod", "Trace from my_mod");
/// trace!("my_mod", "x = {}", val);
/// trace!("my_mod", "x = {val}", val = val);
/// trace!(namespace = "my_mod", "Trace via named ns");
/// trace!(target = "file", "Trace with target");
/// trace!(target = "file", "fmt: {}", val);
/// trace!(target = "file", namespace = "my_mod", "targeted trace");
/// ```
///
/// **Note**: The `target` name must correspond to an existing
/// [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! trace {
  // ── Named-key arms (before any generic $($fmt:tt)+) ──

  (namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().trace_from_static($msg, $namespace);
  };

  (namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .trace(format!($($fmt)+), $namespace);
  };

  (target = $target:literal, namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .trace_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .trace_with_target(format!($($fmt)+), $namespace, $target);
  };

  (target = $target:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .trace_from_static_with_target($msg, env!("CARGO_PKG_NAME"), $target);
  };

  (target = $target:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance().trace_with_target(
      format!($($fmt)+),
      env!("CARGO_PKG_NAME"),
      $target,
    );
  };

  // ── Positional namespace (literal-first) ──
  // Requires two string literals to avoid ambiguity with
  // `trace!("fmt {}", arg)` which must fall through to the
  // single-arg catch-all below.

  ($namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().trace_from_static($msg, $namespace);
  };

  ($namespace:literal, $template:literal, $($arg:tt)*) => {
    $crate::Logger::get_instance()
      .trace(format!($template, $($arg)*), $namespace);
  };

  // ── Default namespace (catch-all, LAST) ──

  ($msg:literal) => {
    $crate::Logger::get_instance()
      .trace_from_static($msg, env!("CARGO_PKG_NAME"));
  };

  ($($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .trace(format!($($fmt)+), env!("CARGO_PKG_NAME"));
  };
}

/// Logs a message at the [`Debug`](crate::Level::Debug) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,no_run
/// # use logger::debug;
/// # // Requires a globally-installed logger.
/// # let state = (42, "ok");
/// # let val = 42;
/// debug!("Debug message");
/// debug!("state = {:?}", state);
/// debug!("my_mod", "Debug from my_mod");
/// debug!("my_mod", "x = {}", val);
/// debug!("my_mod", "x = {val}", val = val);
/// debug!(namespace = "my_mod", "Debug via named ns");
/// debug!(target = "file", "Debug with target");
/// debug!(target = "file", "fmt: {}", val);
/// debug!(target = "file", namespace = "my_mod", "targeted debug");
/// ```
///
/// **Note**: The `target` name must correspond to an existing
/// [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! debug {
  (namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().debug_from_static($msg, $namespace);
  };

  (namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .debug(format!($($fmt)+), $namespace);
  };

  (target = $target:literal, namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .debug_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .debug_with_target(format!($($fmt)+), $namespace, $target);
  };

  (target = $target:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .debug_from_static_with_target($msg, env!("CARGO_PKG_NAME"), $target);
  };

  (target = $target:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance().debug_with_target(
      format!($($fmt)+),
      env!("CARGO_PKG_NAME"),
      $target,
    );
  };

  ($namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().debug_from_static($msg, $namespace);
  };

  ($namespace:literal, $template:literal, $($arg:tt)*) => {
    $crate::Logger::get_instance()
      .debug(format!($template, $($arg)*), $namespace);
  };

  ($msg:literal) => {
    $crate::Logger::get_instance()
      .debug_from_static($msg, env!("CARGO_PKG_NAME"));
  };

  ($($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .debug(format!($($fmt)+), env!("CARGO_PKG_NAME"));
  };
}

/// Logs a message at the [`Info`](crate::Level::Info) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,no_run
/// # use logger::info;
/// # // Requires a globally-installed logger.
/// # let name = "alice";
/// # let n = 42;
/// # let user = "bob";
/// # let addr = "10.0.0.1";
/// info!("Info message");
/// info!("user {} connected", name);
/// info!("my_mod", "Info from my_mod");
/// info!("my_mod", "count = {}", n);
/// info!("my_mod", "user {name} from {ip}", name = user, ip = addr);
/// info!(namespace = "my_mod", "Info via named ns");
/// info!(target = "file", "Info with target");
/// info!(target = "file", "count = {}", n);
/// info!(target = "file", namespace = "my_mod", "targeted info");
/// ```
///
/// **Note**: The `target` name must correspond to an existing
/// [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! info {
  (namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().info_from_static($msg, $namespace);
  };

  (namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .info(format!($($fmt)+), $namespace);
  };

  (target = $target:literal, namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .info_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .info_with_target(format!($($fmt)+), $namespace, $target);
  };

  (target = $target:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .info_from_static_with_target($msg, env!("CARGO_PKG_NAME"), $target);
  };

  (target = $target:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance().info_with_target(
      format!($($fmt)+),
      env!("CARGO_PKG_NAME"),
      $target,
    );
  };

  ($namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().info_from_static($msg, $namespace);
  };

  ($namespace:literal, $template:literal, $($arg:tt)*) => {
    $crate::Logger::get_instance()
      .info(format!($template, $($arg)*), $namespace);
  };

  ($msg:literal) => {
    $crate::Logger::get_instance()
      .info_from_static($msg, env!("CARGO_PKG_NAME"));
  };

  ($($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .info(format!($($fmt)+), env!("CARGO_PKG_NAME"));
  };
}

/// Logs a message at the [`Warn`](crate::Level::Warn) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,no_run
/// # use logger::warn;
/// # // Requires a globally-installed logger.
/// # let pct = 95u8;
/// # let n = 3u32;
/// warn!("Warning message");
/// warn!("disk usage = {}%", pct);
/// warn!("my_mod", "Warning from my_mod");
/// warn!("my_mod", "retries = {}", n);
/// warn!("my_mod", "retry {n}/{max}", n = n, max = 10u32);
/// warn!(namespace = "my_mod", "Warning via named ns");
/// warn!(target = "file", "Warning with target");
/// warn!(target = "file", "disk = {}%", pct);
/// warn!(target = "file", namespace = "my_mod", "targeted warn");
/// ```
///
/// **Note**: The `target` name must correspond to an existing
/// [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! warn {
  (namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().warn_from_static($msg, $namespace);
  };

  (namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .warn(format!($($fmt)+), $namespace);
  };

  (target = $target:literal, namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .warn_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .warn_with_target(format!($($fmt)+), $namespace, $target);
  };

  (target = $target:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .warn_from_static_with_target($msg, env!("CARGO_PKG_NAME"), $target);
  };

  (target = $target:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance().warn_with_target(
      format!($($fmt)+),
      env!("CARGO_PKG_NAME"),
      $target,
    );
  };

  ($namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().warn_from_static($msg, $namespace);
  };

  ($namespace:literal, $template:literal, $($arg:tt)*) => {
    $crate::Logger::get_instance()
      .warn(format!($template, $($arg)*), $namespace);
  };

  ($msg:literal) => {
    $crate::Logger::get_instance()
      .warn_from_static($msg, env!("CARGO_PKG_NAME"));
  };

  ($($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .warn(format!($($fmt)+), env!("CARGO_PKG_NAME"));
  };
}

/// Logs a message at the [`Error`](crate::Level::Error) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,no_run
/// # use logger::error;
/// # // Requires a globally-installed logger.
/// # let e = std::io::Error::new(std::io::ErrorKind::Other, "boom");
/// # let code = 500u16;
/// error!("Error message");
/// error!("io error: {}", e);
/// error!("my_mod", "Error from my_mod");
/// error!("my_mod", "code = {}", code);
/// error!("my_mod", "code = {code}, msg = {msg}", code = code, msg = "err");
/// error!(namespace = "my_mod", "Error via named ns");
/// error!(target = "file", "Error with target");
/// error!(target = "file", "err: {}", e);
/// error!(target = "file", namespace = "my_mod", "targeted error");
/// ```
///
/// **Note**: The `target` name must correspond to an existing
/// [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! error {
  (namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().error_from_static($msg, $namespace);
  };

  (namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .error(format!($($fmt)+), $namespace);
  };

  (target = $target:literal, namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .error_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .error_with_target(format!($($fmt)+), $namespace, $target);
  };

  (target = $target:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .error_from_static_with_target($msg, env!("CARGO_PKG_NAME"), $target);
  };

  (target = $target:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance().error_with_target(
      format!($($fmt)+),
      env!("CARGO_PKG_NAME"),
      $target,
    );
  };

  ($namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().error_from_static($msg, $namespace);
  };

  ($namespace:literal, $template:literal, $($arg:tt)*) => {
    $crate::Logger::get_instance()
      .error(format!($template, $($arg)*), $namespace);
  };

  ($msg:literal) => {
    $crate::Logger::get_instance()
      .error_from_static($msg, env!("CARGO_PKG_NAME"));
  };

  ($($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .error(format!($($fmt)+), env!("CARGO_PKG_NAME"));
  };
}

/// Logs a message at the [`Fatal`](crate::Level::Fatal) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,no_run
/// # use logger::fatal;
/// # // Requires a globally-installed logger.
/// # let reason = "out of memory";
/// # let r = "oom";
/// fatal!("Fatal error");
/// fatal!("cannot continue: {}", reason);
/// fatal!("my_mod", "Fatal from my_mod");
/// fatal!("my_mod", "reason = {}", r);
/// fatal!("my_mod", "reason = {r}, code = {c}", r = r, c = 1u32);
/// fatal!(namespace = "my_mod", "Fatal via named ns");
/// fatal!(target = "file", "Fatal with target");
/// fatal!(target = "file", "reason: {}", r);
/// fatal!(target = "file", namespace = "my_mod", "targeted fatal");
/// ```
///
/// **Note**: The `target` name must correspond to an existing
/// [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! fatal {
  (namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().fatal_from_static($msg, $namespace);
  };

  (namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .fatal(format!($($fmt)+), $namespace);
  };

  (target = $target:literal, namespace = $namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .fatal_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .fatal_with_target(format!($($fmt)+), $namespace, $target);
  };

  (target = $target:literal, $msg:literal) => {
    $crate::Logger::get_instance()
      .fatal_from_static_with_target($msg, env!("CARGO_PKG_NAME"), $target);
  };

  (target = $target:literal, $($fmt:tt)+) => {
    $crate::Logger::get_instance().fatal_with_target(
      format!($($fmt)+),
      env!("CARGO_PKG_NAME"),
      $target,
    );
  };

  ($namespace:literal, $msg:literal) => {
    $crate::Logger::get_instance().fatal_from_static($msg, $namespace);
  };

  ($namespace:literal, $template:literal, $($arg:tt)*) => {
    $crate::Logger::get_instance()
      .fatal(format!($template, $($arg)*), $namespace);
  };

  ($msg:literal) => {
    $crate::Logger::get_instance()
      .fatal_from_static($msg, env!("CARGO_PKG_NAME"));
  };

  ($($fmt:tt)+) => {
    $crate::Logger::get_instance()
      .fatal(format!($($fmt)+), env!("CARGO_PKG_NAME"));
  };
}
