//! Logging macros.
//!
//! Every public macro is a thin wrapper around [`__log!`], which parses the
//! optional `target` / `namespace` keys and forwards the record to the
//! globally installed logger. When no logger has been installed the macros do
//! nothing, so logging before installation never panics.
//!
//! Supported call forms (the same for every level):
//!
//! ```text
//! info!("message");
//! info!("value = {}", 42);
//! info!(namespace = "my_mod", "message");
//! info!(target = "console", "message");
//! info!(target = "console", namespace = "my_mod", "value = {}", 42);
//! ```

#[macro_export]
#[doc(hidden)]
macro_rules! __log {
  ($level:expr; target = $target:literal, namespace = $namespace:literal, $msg:literal)
  => {{
    $crate::__log!(@build $level, $target, $namespace, $msg)
  }};

  ($level:expr; target = $target:literal, namespace = $namespace:literal, $($fmt:tt)+)
  => {{
    $crate::__log!(@build $level, $target, $namespace, format!($($fmt)+))
  }};

  ($level:expr; target = $target:literal, $msg:literal) => {{
    $crate::__log!(@build $level, $target, env!("CARGO_PKG_NAME"), $msg)
  }};

  ($level:expr; target = $target:literal, $($fmt:tt)+) => {{
    $crate::__log!(@build $level, $target, env!("CARGO_PKG_NAME"), format!($($fmt)+))
  }};

  ($level:expr; namespace = $namespace:literal, $msg:literal) => {{
    $crate::__log!(@build $level, $namespace, $msg)
  }};

  ($level:expr; namespace = $namespace:literal, $($fmt:tt)+) => {{
    $crate::__log!(@build $level, $namespace, format!($($fmt)+))
  }};

  ($level:expr; $msg:literal) => {{
    $crate::__log!(@build $level, env!("CARGO_PKG_NAME"), $msg)
  }};

  ($level:expr; $($fmt:tt)+) => {{
    $crate::__log!(@build $level, env!("CARGO_PKG_NAME"), format!($($fmt)+))
  }};

  (@build $level:expr, $target:expr, $namespace:expr, $content:expr) => {{
    let level = $level;
    if let Some(logger) = $crate::Logger::instance() {
      if logger.enabled(level) {
        logger.log($crate::Record::with_target(
          level, $namespace, $target, $content,
        ));
      }
    }
  }};

  (@build $level:expr, $namespace:expr, $content:expr) => {{
    let level = $level;
    if let Some(logger) = $crate::Logger::instance() {
      if logger.enabled(level) {
        logger.log($crate::Record::new(level, $namespace, $content));
      }
    }
  }};
}

/// Logs a message at the [`Trace`](crate::Level::Trace) level.
///
/// Records below the logger's minimum accepted level are skipped at the call
/// site. When no logger has been installed this is a no-op.
///
/// # Examples
///
/// ```rust,no_run
/// use logger::trace;
///
/// trace!("entering the poll loop");
/// trace!(namespace = "depot", "value = {}", 42);
/// ```
#[macro_export]
macro_rules! trace {
  ($($args:tt)*) => {
    $crate::__log!($crate::Level::Trace; $($args)*)
  };
}

/// Logs a message at the [`Debug`](crate::Level::Debug) level.
///
/// Records below the logger's minimum accepted level are skipped at the call
/// site. When no logger has been installed this is a no-op.
///
/// # Examples
///
/// ```rust,no_run
/// use logger::debug;
///
/// debug!("state = {}", "ready");
/// debug!(namespace = "depot", "value = {}", 42);
/// ```
#[macro_export]
macro_rules! debug {
  ($($args:tt)*) => {
    $crate::__log!($crate::Level::Debug; $($args)*)
  };
}

/// Logs a message at the [`Info`](crate::Level::Info) level.
///
/// Records below the logger's minimum accepted level are skipped at the call
/// site. When no logger has been installed this is a no-op.
///
/// # Examples
///
/// ```rust,no_run
/// use logger::info;
///
/// info!("listening on {}", 4030);
/// info!(namespace = "depot", "ready");
/// info!(target = "console", "targeted message");
/// ```
#[macro_export]
macro_rules! info {
  ($($args:tt)*) => {
    $crate::__log!($crate::Level::Info; $($args)*)
  };
}

/// Logs a message at the [`Warn`](crate::Level::Warn) level.
///
/// Records below the logger's minimum accepted level are skipped at the call
/// site. When no logger has been installed this is a no-op.
///
/// # Examples
///
/// ```rust,no_run
/// use logger::warn;
///
/// warn!("retry {n}/{max}", n = 1, max = 3);
/// warn!(namespace = "depot", "degraded");
/// ```
#[macro_export]
macro_rules! warn {
  ($($args:tt)*) => {
    $crate::__log!($crate::Level::Warn; $($args)*)
  };
}

/// Logs a message at the [`Error`](crate::Level::Error) level.
///
/// Records below the logger's minimum accepted level are skipped at the call
/// site. When no logger has been installed this is a no-op.
///
/// # Examples
///
/// ```rust,no_run
/// use logger::error;
///
/// error!("unable to bind: {}", "address in use");
/// error!(namespace = "depot", "connection aborted");
/// ```
#[macro_export]
macro_rules! error {
  ($($args:tt)*) => {
    $crate::__log!($crate::Level::Error; $($args)*)
  };
}

/// Logs a message at the [`Fatal`](crate::Level::Fatal) level.
///
/// Records below the logger's minimum accepted level are skipped at the call
/// site. When no logger has been installed this is a no-op.
///
/// # Examples
///
/// ```rust,no_run
/// use logger::fatal;
///
/// fatal!("cannot continue: {}", "out of memory");
/// fatal!(namespace = "depot", "shutting down");
/// ```
#[macro_export]
macro_rules! fatal {
  ($($args:tt)*) => {
    $crate::__log!($crate::Level::Fatal; $($args)*)
  };
}
